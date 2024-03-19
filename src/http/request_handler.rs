use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::Arc;

use anyhow::Result;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::ServerError;
use hyper::header::{self, CONTENT_TYPE};
use hyper::http::Method;
use hyper::{Body, HeaderMap, Request, Response, StatusCode};
use opentelemetry::trace::SpanKind;
use opentelemetry_semantic_conventions::trace::{HTTP_REQUEST_METHOD, HTTP_ROUTE};
use prometheus::{Encoder, ProtobufEncoder, TextEncoder, PROTOBUF_FORMAT, TEXT_FORMAT};
use serde::de::DeserializeOwned;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::request_context::RequestContext;
use super::telemetry::{get_response_status_code, RequestCounter};
use super::{showcase, telemetry, AppContext};
use crate::async_graphql_hyper::{GraphQLRequestLike, GraphQLResponse};
use crate::blueprint::telemetry::TelemetryExporter;
use crate::blueprint::{is_wildcard, CorsParams};
use crate::config::{PrometheusExporter, PrometheusFormat};

pub const API_URL_PREFIX: &str = "/api";

pub fn graphiql(req: &Request<Body>) -> Result<Response<Body>> {
    let query = req.uri().query();
    let endpoint = "/graphql";
    let endpoint = if let Some(query) = query {
        if query.is_empty() {
            Cow::Borrowed(endpoint)
        } else {
            Cow::Owned(format!("{}?{}", endpoint, query))
        }
    } else {
        Cow::Borrowed(endpoint)
    };

    Ok(Response::new(Body::from(playground_source(
        GraphQLPlaygroundConfig::new(&endpoint).title("Tailcall - GraphQL IDE"),
    ))))
}

fn prometheus_metrics(prometheus_exporter: &PrometheusExporter) -> Result<Response<Body>> {
    let metric_families = prometheus::default_registry().gather();
    let mut buffer = vec![];

    match prometheus_exporter.format {
        PrometheusFormat::Text => TextEncoder::new().encode(&metric_families, &mut buffer)?,
        PrometheusFormat::Protobuf => {
            ProtobufEncoder::new().encode(&metric_families, &mut buffer)?
        }
    };

    let content_type = match prometheus_exporter.format {
        PrometheusFormat::Text => TEXT_FORMAT,
        PrometheusFormat::Protobuf => PROTOBUF_FORMAT,
    };

    Ok(Response::builder()
        .status(200)
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(buffer))?)
}

fn not_found() -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())?)
}

fn create_request_context(req: &Request<Body>, app_ctx: &AppContext) -> RequestContext {
    let upstream = app_ctx.blueprint.upstream.clone();
    let allowed = upstream.allowed_headers;
    let req_headers = create_allowed_headers(req.headers(), &allowed);

    let allowed = app_ctx.blueprint.server.get_experimental_headers();
    let experimental_headers = create_allowed_headers(req.headers(), &allowed);
    RequestContext::from(app_ctx)
        .req_headers(req_headers)
        .experimental_headers(experimental_headers)
}

fn update_cache_control_header(
    response: GraphQLResponse,
    app_ctx: &AppContext,
    req_ctx: Arc<RequestContext>,
) -> GraphQLResponse {
    if app_ctx.blueprint.server.enable_cache_control_header {
        let ttl = req_ctx.get_min_max_age().unwrap_or(0);
        let cache_public_flag = req_ctx.is_cache_public().unwrap_or(true);
        return response.set_cache_control(ttl, cache_public_flag);
    }
    response
}

fn update_experimental_headers(
    response: &mut hyper::Response<hyper::Body>,
    app_ctx: &AppContext,
    req_ctx: Arc<RequestContext>,
) {
    if !app_ctx.blueprint.server.experimental_headers.is_empty() {
        response
            .headers_mut()
            .extend(req_ctx.experimental_headers.clone());
    }
}

pub fn update_response_headers(
    resp: &mut hyper::Response<hyper::Body>,
    cookie_headers: Option<HeaderMap>,
    app_ctx: &AppContext,
) {
    if !app_ctx.blueprint.server.response_headers.is_empty() {
        resp.headers_mut()
            .extend(app_ctx.blueprint.server.response_headers.clone());
    }
    if let Some(cookie_headers) = cookie_headers {
        resp.headers_mut().extend(cookie_headers);
    }
}

#[tracing::instrument(skip_all, fields(otel.name = "graphQL", otel.kind = ?SpanKind::Server))]
pub async fn graphql_request<T: DeserializeOwned + GraphQLRequestLike>(
    req: Request<Body>,
    app_ctx: &AppContext,
    req_counter: &mut RequestCounter,
) -> Result<Response<Body>> {
    req_counter.set_http_route("/graphql");
    let req_ctx = Arc::new(create_request_context(&req, app_ctx));
    let bytes = hyper::body::to_bytes(req.into_body()).await?;
    let graphql_request = serde_json::from_slice::<T>(&bytes);
    match graphql_request {
        Ok(request) => {
            let mut response = request.data(req_ctx.clone()).execute(&app_ctx.schema).await;
            let cookie_headers = req_ctx.cookie_headers.clone();
            response = update_cache_control_header(response, app_ctx, req_ctx.clone());
            let mut resp = response.to_response()?;
            update_response_headers(
                &mut resp,
                cookie_headers.map(|v| v.lock().unwrap().clone()),
                app_ctx,
            );
            update_experimental_headers(&mut resp, app_ctx, req_ctx);
            Ok(resp)
        }
        Err(err) => {
            tracing::error!(
                "Failed to parse request: {}",
                String::from_utf8(bytes.to_vec()).unwrap()
            );

            let mut response = async_graphql::Response::default();
            let server_error =
                ServerError::new(format!("Unexpected GraphQL Request: {}", err), None);
            response.errors = vec![server_error];

            Ok(GraphQLResponse::from(response).to_response()?)
        }
    }
}

fn create_allowed_headers(headers: &HeaderMap, allowed: &BTreeSet<String>) -> HeaderMap {
    let mut new_headers = HeaderMap::new();
    for (k, v) in headers.iter() {
        if allowed
            .iter()
            .any(|allowed_key| allowed_key.eq_ignore_ascii_case(k.as_str()))
        {
            new_headers.insert(k, v.clone());
        }
    }
    new_headers
}

fn ensure_usable_cors_rules(layer: &CorsParams) {
    if layer.allow_credentials {
        assert!(
            layer
                .allow_headers
                .as_ref()
                .filter(|val| is_wildcard(val))
                .is_none(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Allow-Headers: *`"
        );

        assert!(
            layer
                .allow_methods
                .as_ref()
                .filter(|val| is_wildcard(val))
                .is_none(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Allow-Methods: *`"
        );

        assert!(
            layer
                .allow_origin
                .first()
                .filter(|val| is_wildcard(val))
                .is_none(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Allow-Origin: *`"
        );

        assert!(
            !layer.expose_headers_is_wildcard(),
            "Invalid CORS configuration: Cannot combine `Access-Control-Allow-Credentials: true` \
             with `Access-Control-Expose-Headers: *`"
        );
    }
}

pub async fn handle_request_with_cors<T: DeserializeOwned + GraphQLRequestLike>(
    req: Request<Body>,
    cors: &CorsParams,
    app_ctx: Arc<AppContext>,
    request_counter: &mut RequestCounter,
) -> Result<Response<Body>> {
    ensure_usable_cors_rules(cors);
    let (parts, body) = req.into_parts();
    let origin = parts.headers.get(&header::ORIGIN);

    let mut headers = HeaderMap::new();

    // These headers are applied to both preflight and subsequent regular CORS
    // requests: https://fetch.spec.whatwg.org/#http-responses

    headers.extend(cors.allow_origin_to_header(origin));
    headers.extend(cors.allow_credentials_to_header());
    headers.extend(cors.allow_private_network_to_header(&parts));
    headers.extend(cors.vary_to_header());

    // Return results immediately upon preflight request
    if parts.method == Method::OPTIONS {
        // These headers are applied only to preflight requests
        headers.extend(cors.allow_methods_to_header());
        headers.extend(cors.allow_headers_to_header());
        headers.extend(cors.max_age_to_header());

        let mut response = Response::new(Body::default());
        std::mem::swap(response.headers_mut(), &mut headers);

        Ok(response)
    } else {
        // This header is applied only to non-preflight requests
        headers.extend(cors.expose_headers_to_header());

        let req = Request::from_parts(parts, body);
        let mut response = handle_request_inner::<T>(req, app_ctx, request_counter).await?;

        let response_headers = response.headers_mut();

        // vary header can have multiple values, don't overwrite
        // previously-set value(s).
        if let Some(vary) = headers.remove(header::VARY) {
            response_headers.append(header::VARY, vary);
        }
        // extend will overwrite previous headers of remaining names
        response_headers.extend(headers.drain());

        Ok(response)
    }
}

async fn handle_rest_apis(
    mut request: Request<Body>,
    app_ctx: Arc<AppContext>,
    req_counter: &mut RequestCounter,
) -> Result<Response<Body>> {
    *request.uri_mut() = request.uri().path().replace(API_URL_PREFIX, "").parse()?;
    let req_ctx = Arc::new(create_request_context(&request, app_ctx.as_ref()));
    if let Some(p_request) = app_ctx.endpoints.matches(&request) {
        let http_route = format!("{API_URL_PREFIX}{}", p_request.path.as_str());
        req_counter.set_http_route(&http_route);
        let span = tracing::info_span!(
            "REST",
            otel.name = format!("REST {} {}", request.method(), p_request.path.as_str()),
            otel.kind = ?SpanKind::Server,
            { HTTP_REQUEST_METHOD } = %request.method(),
            { HTTP_ROUTE } = http_route
        );
        return async {
            let graphql_request = p_request.into_request(request).await?;
            let mut response = graphql_request
                .data(req_ctx.clone())
                .execute(&app_ctx.schema)
                .await;
            let cookie_headers = req_ctx.cookie_headers.clone();
            response = update_cache_control_header(response, app_ctx.as_ref(), req_ctx.clone());
            let mut resp = response.to_rest_response()?;
            update_response_headers(
                &mut resp,
                cookie_headers.map(|v| v.lock().unwrap().clone()),
                app_ctx.as_ref(),
            );
            update_experimental_headers(&mut resp, app_ctx.as_ref(), req_ctx);
            Ok(resp)
        }
        .instrument(span)
        .await;
    }

    not_found()
}

async fn handle_request_inner<T: DeserializeOwned + GraphQLRequestLike>(
    req: Request<Body>,
    app_ctx: Arc<AppContext>,
    req_counter: &mut RequestCounter,
) -> Result<Response<Body>> {
    if req.uri().path().starts_with(API_URL_PREFIX) {
        return handle_rest_apis(req, app_ctx, req_counter).await;
    }

    match *req.method() {
        // NOTE:
        // The first check for the route should be for `/graphql`
        // This is always going to be the most used route.
        hyper::Method::POST if req.uri().path() == "/graphql" => {
            graphql_request::<T>(req, app_ctx.as_ref(), req_counter).await
        }
        hyper::Method::POST
            if app_ctx.blueprint.server.enable_showcase
                && req.uri().path() == "/showcase/graphql" =>
        {
            let app_ctx =
                match showcase::create_app_ctx::<T>(&req, app_ctx.runtime.clone(), false).await? {
                    Ok(app_ctx) => app_ctx,
                    Err(res) => return Ok(res),
                };

            graphql_request::<T>(req, &app_ctx, req_counter).await
        }

        hyper::Method::GET => {
            if let Some(TelemetryExporter::Prometheus(prometheus)) =
                app_ctx.blueprint.telemetry.export.as_ref()
            {
                if req.uri().path() == prometheus.path {
                    return prometheus_metrics(prometheus);
                }
            };

            if app_ctx.blueprint.server.enable_graphiql {
                return graphiql(&req);
            }

            not_found()
        }
        _ => not_found(),
    }
}

#[tracing::instrument(
    skip_all,
    err,
    fields(
        otel.name = "request",
        otel.kind = ?SpanKind::Server,
        url.path = %req.uri().path(),
        http.request.method = %req.method()
    )
)]
pub async fn handle_request<T: DeserializeOwned + GraphQLRequestLike>(
    req: Request<Body>,
    app_ctx: Arc<AppContext>,
) -> Result<Response<Body>> {
    telemetry::propagate_context(&req);
    let mut req_counter = RequestCounter::new(&app_ctx.blueprint.telemetry, &req);

    let response = if let Some(cors_params) = app_ctx.blueprint.server.cors_params.clone() {
        handle_request_with_cors::<T>(req, &cors_params, app_ctx, &mut req_counter).await
    } else {
        handle_request_inner::<T>(req, app_ctx, &mut req_counter).await
    };

    req_counter.update(&response);
    if let Ok(response) = &response {
        let status = get_response_status_code(response);
        tracing::Span::current().set_attribute(status.key, status.value);
    };

    response
}

#[cfg(test)]
mod test {
    #[test]
    fn test_create_allowed_headers() {
        use std::collections::BTreeSet;

        use hyper::header::{HeaderMap, HeaderValue};

        use super::create_allowed_headers;

        let mut headers = HeaderMap::new();
        headers.insert("X-foo", HeaderValue::from_static("bar"));
        headers.insert("x-bar", HeaderValue::from_static("foo"));
        headers.insert("x-baz", HeaderValue::from_static("baz"));

        let allowed = BTreeSet::from_iter(vec!["x-foo".to_string(), "X-bar".to_string()]);

        let new_headers = create_allowed_headers(&headers, &allowed);
        assert_eq!(new_headers.len(), 2);
        assert_eq!(new_headers.get("x-foo").unwrap(), "bar");
        assert_eq!(new_headers.get("x-bar").unwrap(), "foo");
    }
}
