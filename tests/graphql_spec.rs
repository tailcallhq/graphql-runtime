use std::fmt::Debug;
#[cfg(test)]
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Once};

use async_graphql::parser::types::TypeSystemDefinition;
use async_graphql::Request;
use derive_setters::Setters;
use hyper::http::{HeaderName, HeaderValue};
use hyper::HeaderMap;
use pretty_assertions::assert_eq;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tailcall::blueprint::Blueprint;
use tailcall::config::Config;
use tailcall::directive::DirectiveCodec;
use tailcall::http::{RequestContext, ServerContext};
use tailcall::print_schema;
use tailcall::valid::{Cause, NeoValid, ValidExtensions};
mod graphql_mock;

static INIT: Once = Once::new();

#[derive(Debug, Default, Setters)]
struct GraphQLSpec {
  path: PathBuf,
  client_sdl: String,
  server_sdl: Vec<String>,
  merged_server_sdl: String,
  sdl_errors: Vec<SDLError>,
  test_queries: Vec<GraphQLQuerySpec>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
struct SDLError {
  message: String,
  trace: Vec<String>,
}

impl<'a> From<Cause<&'a str>> for SDLError {
  fn from(value: Cause<&'a str>) -> Self {
    SDLError { message: value.message.to_string(), trace: value.trace.iter().map(|e| e.to_string()).collect() }
  }
}

impl From<Cause<String>> for SDLError {
  fn from(value: Cause<String>) -> Self {
    SDLError { message: value.message.to_string(), trace: value.trace.iter().map(|e| e.to_string()).collect() }
  }
}

#[derive(Debug, Default)]
struct GraphQLQuerySpec {
  query: String,
  expected: Value,
}

impl GraphQLSpec {
  fn query(mut self, query: String, expected: Value) -> Self {
    self.test_queries.push(GraphQLQuerySpec { query, expected });
    self
  }

  fn new(path: PathBuf, content: &str) -> GraphQLSpec {
    INIT.call_once(|| {
      env_logger::builder()
        .filter(Some("graphql_spec"), log::LevelFilter::Info)
        .init();
    });

    let mut spec = GraphQLSpec::default().path(path);
    let mut server_sdl = Vec::new();
    for component in content.split("#>") {
      if component.contains(CLIENT_SDL) {
        let trimmed = component.replace(CLIENT_SDL, "").trim().to_string();

        // Extract all errors
        if trimmed.contains("@error") {
          let doc = async_graphql::parser::parse_schema(trimmed.as_str()).unwrap();
          for def in doc.definitions {
            if let TypeSystemDefinition::Type(type_def) = def {
              for dir in type_def.node.directives {
                if dir.node.name.node == "error" {
                  spec
                    .sdl_errors
                    .push(SDLError::from_directive(&dir.node).to_result().unwrap());
                }
              }
            }
          }
        }

        spec = spec.client_sdl(trimmed);
      }
      if component.contains(SERVER_SDL) {
        server_sdl.push(component.replace(SERVER_SDL, "").trim().to_string());
        spec = spec.server_sdl(server_sdl.clone());
      }
      if component.contains(MERGED_SDL) {
        spec = spec.merged_server_sdl(component.replace(MERGED_SDL, "").trim().to_string());
      }
      if component.contains(CLIENT_QUERY) {
        let regex = Regex::new(r"@expect.*\) ").unwrap();
        let query_string = component.replace(CLIENT_QUERY, "");
        let parsed_query = async_graphql::parser::parse_query(query_string.clone()).unwrap();

        let query_string = regex.replace_all(query_string.as_str(), "");
        let query_string = query_string.trim();
        for (_, q) in parsed_query.operations.iter() {
          let expect = q.node.directives.iter().find(|d| d.node.name.node == "expect");
          assert!(
            expect.is_some(),
            "@expect directive is required in query:\n```\n{}\n```",
            query_string
          );
          if let Some(dir) = expect {
            let expected = dir
              .node
              .arguments
              .iter()
              .find(|a| a.0.node == "json")
              .map(|a| a.clone().1.node.into_json().unwrap())
              .unwrap();
            spec = spec.query(query_string.to_string(), expected);
          }
        }
      }
    }
    spec
  }

  fn cargo_read(path: &str) -> std::io::Result<Vec<GraphQLSpec>> {
    let mut dir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir_path.push(path);

    let entries = fs::read_dir(dir_path.clone())?;
    let mut files = Vec::new();
    for entry in entries {
      let path = entry?.path();
      if path.is_file() && path.extension().unwrap_or_default() == "graphql" {
        let contents = fs::read_to_string(path.clone())?;
        let path_buf = path.clone();
        files.push(GraphQLSpec::new(path_buf, contents.as_str()));
      }
    }

    assert!(
      !files.is_empty(),
      "No files found in {}",
      dir_path.to_str().unwrap_or_default()
    );
    Ok(files)
  }
}

const CLIENT_SDL: &str = "client-sdl";
const SERVER_SDL: &str = "server-sdl";
const CLIENT_QUERY: &str = "client-query";
const MERGED_SDL: &str = "merged-sdl";

// Check if SDL -> Config -> SDL is identity
#[test]
fn test_config_identity() -> std::io::Result<()> {
  let specs = GraphQLSpec::cargo_read("tests/graphql");

  for spec in specs? {
    let content = spec.server_sdl[0].as_str();
    let expected = content;

    let config = Config::from_sdl(content).to_result().unwrap();
    let actual = config.to_sdl();
    assert_eq!(actual, expected, "ServerSDLIdentity: {}", spec.path.display());
    log::info!("ServerSDLIdentity: {} ... ok", spec.path.display());
  }

  Ok(())
}

// Check server SDL matches expected client SDL
#[test]
fn test_server_to_client_sdl() -> std::io::Result<()> {
  let specs = GraphQLSpec::cargo_read("tests/graphql");

  for spec in specs? {
    let expected = spec.client_sdl;
    let content = spec.server_sdl[0].as_str();
    let config = Config::from_sdl(content).to_result().unwrap();
    let actual = print_schema::print_schema((Blueprint::try_from(&config).unwrap()).to_schema());
    assert_eq!(actual, expected, "ClientSDL: {}", spec.path.display());
    log::info!("ClientSDL: {} ... ok", spec.path.display());
  }

  Ok(())
}

// Check if execution gives expected response
#[tokio::test]
async fn test_execution() -> std::io::Result<()> {
  let mut mock_server = graphql_mock::start_mock_server();
  graphql_mock::setup_mocks(&mut mock_server);

  let specs = GraphQLSpec::cargo_read("tests/graphql/passed");

  for spec in specs? {
    let mut config = Config::from_sdl(&spec.server_sdl[0]).to_result().unwrap();
    config.server.enable_query_validation = Some(false);

    let blueprint = Blueprint::try_from(&config)
      .trace(spec.path.to_str().unwrap_or_default())
      .unwrap();
    let server_ctx = ServerContext::new(blueprint);
    let schema = server_ctx.schema.clone();

    for q in spec.test_queries {
      let mut headers = HeaderMap::new();
      headers.insert(HeaderName::from_static("authorization"), HeaderValue::from_static("1"));
      let req_ctx = Arc::new(RequestContext::from(&server_ctx).req_headers(headers));
      let req = Request::from(q.query.as_str()).data(req_ctx.clone());
      let res = schema.execute(req).await;
      let json = serde_json::to_string(&res).unwrap();
      let expected = serde_json::to_string(&q.expected).unwrap();
      assert_eq!(json, expected, "QueryExecution: {}", spec.path.display());
      log::info!("QueryExecution: {} ... ok", spec.path.display());
    }
  }

  Ok(())
}

// Standardize errors on Client SDL
#[test]
fn test_failures_in_client_sdl() -> std::io::Result<()> {
  let specs = GraphQLSpec::cargo_read("tests/graphql/errors");

  for spec in specs? {
    let expected = spec.sdl_errors;
    let content = spec.server_sdl[0].as_str();
    let config = Config::from_sdl(content);

    let actual = config
      .and_then(|config| NeoValid::from(Blueprint::try_from(&config)))
      .to_result();
    match actual {
      Err(cause) => {
        let actual: Vec<SDLError> = cause.as_vec().iter().map(|e| e.to_owned().into()).collect();
        assert_eq!(actual, expected, "Server SDL failure mismatch: {}", spec.path.display());
        log::info!("ClientSDLError: {} ... ok", spec.path.display());
      }
      _ => panic!("ClientSDLError: {}", spec.path.display()),
    }
  }

  Ok(())
}

#[test]
fn test_merge_sdl() -> std::io::Result<()> {
  let specs = GraphQLSpec::cargo_read("tests/graphql/merge");

  for spec in specs? {
    let expected = spec.merged_server_sdl;
    let content = spec
      .server_sdl
      .iter()
      .map(|s| Config::from_sdl(s.as_str()).to_result().unwrap())
      .collect::<Vec<_>>();
    let config = content.iter().fold(Config::default(), |acc, c| acc.merge_right(c));
    let actual = config.to_sdl();
    assert_eq!(actual, expected, "SDLMerge: {}", spec.path.display());
    log::info!("SDLMerge: {} ... ok", spec.path.display());
  }

  Ok(())
}
