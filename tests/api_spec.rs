#[cfg(test)]
mod test {
  use std::collections::BTreeMap;
  use std::fs;
  use std::str::FromStr;
  use std::sync::Arc;

  use async_graphql_value::ConstValue;
  use http_cache_semantics::RequestLike;
  use hyper::{Body, Request};
  use reqwest::header::{HeaderName, HeaderValue};
  use serde::Deserialize;
  use serde_json::Value;
  use tailcall::blueprint::Blueprint;
  use tailcall::config::Config;
  use tailcall::http::{graphql_request, HttpClient, Method, Response, ServerContext};
  use url::Url;

  #[derive(Deserialize, Clone, Debug, Default)]
  pub struct APIRequest {
    method: Option<Method>,
    pub url: Option<Url>,
    pub headers: Option<BTreeMap<String, String>>,
    pub body: Option<serde_json::Value>,
  }
  #[derive(Deserialize, Clone, Debug)]
  pub struct APIResponse {
    pub status: u16,
    pub headers: Option<BTreeMap<String, String>>,
    pub body: Option<serde_json::Value>,
  }
  #[derive(Deserialize, Clone, Debug)]
  pub struct UpstreamRequest(pub APIRequest);
  #[derive(Deserialize, Clone, Debug)]
  pub struct UpstreamResponse(APIResponse);
  #[derive(Deserialize, Clone)]
  pub struct DownstreamRequest(pub APIRequest);
  #[derive(Deserialize, Clone)]
  pub struct DownstreamResponse(pub APIResponse);
  #[derive(Deserialize, Clone)]
  pub struct DownstreamAssertion {
    pub request: DownstreamRequest,
    pub response: DownstreamResponse,
  }

  #[derive(Default, Deserialize, Clone)]
  pub struct HttpSpec {
    pub config: String,
    pub name: String,
    pub description: Option<String>,
    pub upstream_mocks: Vec<(UpstreamRequest, UpstreamResponse)>,
    pub expected_upstream_requests: Vec<UpstreamRequest>,
    pub downstream_assertions: Vec<DownstreamAssertion>,
  }

  async fn read_config_from_path(config_path: &str) -> Option<Config> {
    Config::from_file_paths([config_path.to_string()].iter()).await.ok()
  }

  impl HttpSpec {
    fn read(spec: &str) -> Option<Self> {
      spec
        .split('.')
        .last()
        .and_then(|ext| match ext.to_lowercase().as_str() {
          "json" => Self::read_json(spec),
          "yaml" => Self::read_yaml(spec),
          _ => None,
        })
    }
    fn read_json(spec: &str) -> Option<Self> {
      let contents = fs::read_to_string(spec).ok()?;
      let spec = serde_json::from_str(&contents);

      spec.ok()
    }

    fn read_yaml(spec: &str) -> Option<Self> {
      let contents = fs::read_to_string(spec).ok()?;
      let spec = serde_yaml::from_str(&contents);
      spec.ok()
    }
    async fn setup(&self) -> Arc<ServerContext> {
      let config = read_config_from_path(self.config.clone().as_str()).await.unwrap();
      let blueprint = Blueprint::try_from(&config).unwrap();
      let client = Arc::new(MockHttpClient { upstream_mocks: Arc::new(self.upstream_mocks.to_vec()) });
      let server_context = ServerContext::new(blueprint, client);
      Arc::new(server_context)
    }
  }

  #[derive(Clone)]
  struct MockHttpClient {
    upstream_mocks: Arc<Vec<(UpstreamRequest, UpstreamResponse)>>,
  }
  #[async_trait::async_trait]
  impl HttpClient for MockHttpClient {
    async fn execute(&self, req: reqwest::Request) -> anyhow::Result<Response> {
      let upstream_mocks = self.upstream_mocks.clone();
      let upstream_mock = upstream_mocks.iter().find(|(upstream_request, _res)| {
        let method_match = req.method().as_str()
          == serde_json::to_string(&upstream_request.0.method.clone().unwrap_or_default())
            .unwrap()
            .as_str()
            .trim_matches('"');
        let url_match = req.url().as_str() == req.uri().to_string().as_str();
        method_match && url_match
      });
      let upstream_response = upstream_mock.unwrap().clone().1.clone();
      let mut response =
        Response { status: reqwest::StatusCode::from_u16(upstream_response.0.status).unwrap(), ..Default::default() };
      let headers = upstream_response.0.headers.unwrap_or_default();
      for (k, v) in headers.iter() {
        response.headers.insert(
          HeaderName::from_str(k.as_str()).unwrap(),
          HeaderValue::from_str(v.as_str()).unwrap(),
        );
      }
      match upstream_response.0.body {
        None => {
          response.body = ConstValue::Null;
        }
        Some(a) => {
          response.body = ConstValue::try_from(serde_json::from_value::<Value>(a).unwrap())?;
        }
      }
      Ok(response)
    }
  }

  #[tokio::test]
  async fn test_body_yaml() {
    let spec = HttpSpec::read("tests/data/sample.yaml").unwrap();
    for downstream_assertion in spec.clone().downstream_assertions.iter() {
      let response = run(spec.clone(), &downstream_assertion).await;
      let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
      assert_eq!(
        body,
        serde_json::to_string(&downstream_assertion.response.0.body).unwrap()
      );
    }
  }

  #[tokio::test]
  async fn test_body_json() {
    let spec = HttpSpec::read("tests/data/sample.json").unwrap();
    for downstream_assertion in spec.clone().downstream_assertions.iter() {
      let response = run(spec.clone(), &downstream_assertion).await;
      let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
      assert_eq!(
        body,
        serde_json::to_string(&downstream_assertion.response.0.body).unwrap()
      );
    }
  }

  async fn run(spec: HttpSpec, downstream_assertion: &&DownstreamAssertion) -> hyper::Response<Body> {
    let query_string = serde_json::to_string(&downstream_assertion.request.0.body).unwrap();
    let method = downstream_assertion.request.0.method.clone().unwrap_or_default();
    let url = downstream_assertion
      .request
      .0
      .url
      .clone()
      .unwrap_or(Url::parse("http://localhost:8080/graphql").unwrap());
    let state = spec.setup().await;
    let req = Request::builder()
      .method(method)
      .uri(url.as_str())
      .body(Body::from(query_string))
      .unwrap();
    graphql_request(req, state.as_ref()).await.unwrap()
  }
}
