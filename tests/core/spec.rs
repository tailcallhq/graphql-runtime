extern crate core;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::{fs, panic};

use anyhow::Context;
use colored::Colorize;
use futures_util::future::join_all;
use http::Request;
use hyper::Body;
use serde::{Deserialize, Serialize};
use tailcall::core::app_context::AppContext;
use tailcall::core::async_graphql_hyper::{GraphQLBatchRequest, GraphQLRequest};
use tailcall::core::blueprint::{Blueprint, BlueprintError};
use tailcall::core::config::reader::ConfigReader;
use tailcall::core::config::transformer::Required;
use tailcall::core::config::{Config, ConfigModule, Source};
use tailcall::core::http::handle_request;
use tailcall::core::print_schema::print_schema;
use tailcall::core::variance::Invariant;
use tailcall_prettier::Parser;
use tailcall_valid::{Cause, Valid, ValidationError, Validator};

use super::file::File;
use super::http::Http;
use super::model::*;
use super::runtime::ExecutionSpec;
use crate::core::runtime;

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
struct SDLError {
    message: String,
    trace: Vec<String>,
    description: Option<String>,
}

impl<'a> From<Cause<&'a str>> for SDLError {
    fn from(value: Cause<&'a str>) -> Self {
        SDLError {
            message: value.message.to_string(),
            trace: value.trace.iter().map(|e| e.to_string()).collect(),
            description: None,
        }
    }
}

impl From<Cause<String>> for SDLError {
    fn from(value: Cause<String>) -> Self {
        SDLError {
            message: value.message.to_string(),
            trace: value.trace.iter().map(|e| e.to_string()).collect(),
            description: value.description,
        }
    }
}

async fn is_sdl_error(spec: &ExecutionSpec, config_module: Valid<ConfigModule, String>) -> bool {
    if spec.sdl_error {
        // errors: errors are expected, make sure they match
        let blueprint = config_module.and_then(|cfg| match Blueprint::try_from(&cfg) {
            Ok(blueprint) => Valid::succeed(blueprint),
            Err(e) => Valid::from_validation_err(BlueprintError::to_validation_string(e)),
        });

        match blueprint.to_result() {
            Ok(_) => {
                tracing::error!("\terror FAIL");
                panic!(
                    "Spec {} {:?} with \"sdl error\" directive did not have a validation error.",
                    spec.name, spec.path
                );
            }
            Err(error) => {
                let errors: Vec<SDLError> =
                    error.as_vec().iter().map(|e| e.to_owned().into()).collect();

                let snapshot_name = format!("{}_error", spec.safe_name);

                insta::assert_json_snapshot!(snapshot_name, errors);
            }
        };

        return true;
    }
    false
}

async fn check_identity(spec: &ExecutionSpec) {
    // TODO: we should probably figure out a way to do this for every test
    // but GraphQL identity checking is very hard, since a lot depends on the code
    // style the re-serializing check gives us some of the advantages of the
    // identity check too, but we are missing out on some by having it only
    // enabled for either new tests that request it or old graphql_spec
    // tests that were explicitly written with it in mind
    if spec.check_identity {
        for (source, content) in spec.server.iter() {
            if matches!(source, Source::GraphQL) {
                let config = Config::from_source(source.to_owned(), content).unwrap();
                let actual = config.to_sdl();

                // \r is added automatically in windows, it's safe to replace it with \n
                let content = content.replace("\r\n", "\n");

                let path_str = spec.path.display().to_string();
                let context = format!("path: {}", path_str);

                let actual = tailcall_prettier::format(actual, &tailcall_prettier::Parser::Gql)
                    .await
                    .map_err(|e| e.with_context(context.clone()))
                    .unwrap();

                let expected = tailcall_prettier::format(content, &tailcall_prettier::Parser::Gql)
                    .await
                    .map_err(|e| e.with_context(context.clone()))
                    .unwrap();

                pretty_assertions::assert_eq!(
                    actual,
                    expected,
                    "Identity check failed for {:#?}",
                    spec.path,
                );
            } else {
                panic!(
                    "Spec {:#?} has \"check identity\" enabled, but its config isn't in GraphQL.",
                    spec.path
                );
            }
        }
    }
}

async fn run_query_tests_on_spec(
    spec: ExecutionSpec,
    server: Vec<ConfigModule>,
    mock_http_client: Arc<Http>,
) {
    if let Some(tests) = spec.test.as_ref() {
        let app_ctx = spec
            .app_context(
                server.first().unwrap(),
                spec.env.clone().unwrap_or_default(),
                mock_http_client.clone(),
            )
            .await;

        // test: Run test specs

        for (i, test) in tests.iter().enumerate() {
            let response = run_test(app_ctx.clone(), test)
                .await
                .context(spec.path.to_str().unwrap().to_string())
                .unwrap();

            let mut headers: BTreeMap<String, String> = BTreeMap::new();

            for (key, value) in response.headers() {
                headers.insert(key.to_string(), value.to_str().unwrap().to_string());
            }

            let response: APIResponse = APIResponse {
                status: response.status().clone().as_u16(),
                headers,
                body: Some(APIBody::Value(
                    serde_json::from_slice(
                        &hyper::body::to_bytes(response.into_body()).await.unwrap(),
                    )
                    .unwrap_or_default(),
                )),
            };

            let snapshot_name = format!("{}_{}", spec.safe_name, i);

            insta::assert_json_snapshot!(snapshot_name, response);
        }

        mock_http_client.test_hits(&spec.path);
    }
}

async fn test_spec(spec: ExecutionSpec) {
    // insta settings
    let mut insta_settings = insta::Settings::clone_current();
    insta_settings.set_prepend_module_to_snapshot(false);
    let _guard = insta::Settings::bind_to_scope(&insta_settings);

    let mock_http_client = Arc::new(Http::new(&spec));

    let mut runtime = runtime::create_runtime(mock_http_client.clone(), spec.env.clone(), None);
    runtime.file = Arc::new(File::new(spec.clone()));
    let reader = ConfigReader::init(runtime);

    // Resolve all configs
    let config_modules = join_all(spec.server.iter().map(|(source, content)| async {
        let config = Config::from_source(source.to_owned(), content)?;

        reader.resolve(config, spec.path.parent()).await
    }))
    .await;

    let config_module = Valid::from_iter(config_modules.iter(), |config_module| {
        Valid::from(config_module.as_ref().map_err(|e| {
            match e.downcast_ref::<ValidationError<String>>() {
                Some(err) => err.clone(),
                None => ValidationError::new(e.to_string()),
            }
        }))
    })
    .and_then(|cfgs| {
        cfgs.into_iter()
            .fold(Valid::succeed(ConfigModule::default()), |acc, c| {
                acc.and_then(|acc| acc.unify(c.clone()))
            })
    })
    // Apply required transformers to the configuration
    .and_then(|cfg| cfg.transform(Required));

    // check sdl error if any
    if is_sdl_error(&spec, config_module.clone()).await {
        return;
    }

    let merged = config_module.to_result().unwrap().to_sdl();

    let formatter = tailcall_prettier::format(merged, &Parser::Gql)
        .await
        .unwrap();

    let snapshot_name = format!("{}_merged", spec.safe_name);

    insta::assert_snapshot!(snapshot_name, formatter);

    let config_modules = config_modules
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    check_identity(&spec).await;

    // client: Check if client spec matches snapshot
    if config_modules.len() == 1 {
        let config = &config_modules[0];

        let client = print_schema(
            (Blueprint::try_from(config)
                .context(format!("file: {}", spec.path.to_str().unwrap()))
                .unwrap())
            .to_schema(),
        );

        let formatted = tailcall_prettier::format(client, &Parser::Gql)
            .await
            .unwrap();
        let snapshot_name = format!("{}_client", spec.safe_name);

        insta::assert_snapshot!(snapshot_name, formatted);
    }

    // run query tests
    run_query_tests_on_spec(spec, config_modules, mock_http_client).await;
}

pub async fn load_and_test_execution_spec(path: &Path) -> anyhow::Result<()> {
    let contents = fs::read_to_string(path)?;
    let spec: ExecutionSpec = ExecutionSpec::from_source(path, contents)
        .await
        .with_context(|| path.display().to_string())?;

    match spec.runner {
        Some(Annotation::Skip) => {
            println!("{} ... {}", spec.path.display(), "skipped".blue());
        }
        None => test_spec(spec).await,
    }

    Ok(())
}

async fn run_test(
    app_ctx: Arc<AppContext>,
    request: &APIRequest,
) -> anyhow::Result<http::Response<Body>> {
    let body = request
        .body
        .as_ref()
        .map(|body| Body::from(body.to_bytes()))
        .unwrap_or_default();

    let method = request.method.clone();
    let headers = request.headers.clone();
    let url = request.url.clone();
    let req = headers
        .into_iter()
        .fold(
            Request::builder()
                .method(method.to_hyper())
                .uri(url.as_str()),
            |acc, (key, value)| acc.header(key, value),
        )
        .body(body)?;

    // TODO: reuse logic from server.rs to select the correct handler
    if app_ctx.blueprint.server.enable_batch_requests {
        handle_request::<GraphQLBatchRequest>(req, app_ctx).await
    } else {
        handle_request::<GraphQLRequest>(req, app_ctx).await
    }
}
