use std::collections::BTreeMap;
use std::net::{AddrParseError, IpAddr};

use derive_setters::Setters;
use hyper::header::{HeaderName, HeaderValue};
use hyper::HeaderMap;

use crate::config;
use crate::valid::{NeoValid, ValidationError};

#[derive(Clone, Debug, Setters)]
pub struct Server {
  pub enable_apollo_tracing: bool,
  pub enable_cache_control_header: bool,
  pub enable_graphiql: Option<String>,
  pub enable_introspection: bool,
  pub enable_query_validation: bool,
  pub enable_response_validation: bool,
  pub global_response_timeout: i64,
  pub port: u16,
  pub hostname: IpAddr,
  pub vars: BTreeMap<String, String>,
  pub response_headers: HeaderMap,
}

impl Server {
  pub fn get_enable_http_validation(&self) -> bool {
    self.enable_response_validation
  }
  pub fn get_enable_cache_control(&self) -> bool {
    self.enable_cache_control_header
  }

  pub fn get_enable_introspection(&self) -> bool {
    self.enable_introspection
  }

  pub fn get_enable_query_validation(&self) -> bool {
    self.enable_query_validation
  }
}

impl TryFrom<crate::config::Server> for Server {
  type Error = ValidationError<String>;

  fn try_from(config_server: config::Server) -> Result<Self, Self::Error> {
    configure_server(&config_server).to_result()
  }
}

fn validate_hostname(hostname: String) -> NeoValid<IpAddr, String> {
  if hostname == "localhost" {
    NeoValid::succeed(IpAddr::from([127, 0, 0, 1]))
  } else {
    NeoValid::from(
      hostname
        .parse()
        .map_err(|e: AddrParseError| ValidationError::new(format!("Parsing failed because of {}", e))),
    )
    .trace("hostname")
    .trace("@server")
    .trace("schema")
  }
}

const RESTRICTED_ROUTES: &[&str] = &["/", "/graphql"];

fn handle_graphiql(graphiql: Option<String>) -> NeoValid<Option<String>, String> {
  let mut graph = None;
  if let Some(enable_graphiql) = graphiql.clone() {
    let lowered_route = enable_graphiql.to_lowercase();
    if RESTRICTED_ROUTES.contains(&lowered_route.as_str()) {
      return NeoValid::from_validation_err(
        ValidationError::new(format!(
          "Cannot use restricted routes '{}' for enabling graphiql",
          enable_graphiql
        ))
        .trace("enableGraphiql")
        .trace("@server")
        .trace("schema"),
      );
    } else {
      graph = Some(enable_graphiql);
    }
  };
  NeoValid::succeed(graph)
}

fn handle_response_headers(resp_headers: BTreeMap<String, String>) -> NeoValid<HeaderMap, String> {
  NeoValid::from_iter(resp_headers.iter(), |(k, v)| {
    let name = NeoValid::from(
      HeaderName::from_bytes(k.as_bytes())
        .map_err(|e| ValidationError::new(format!("Parsing failed because of {}", e))),
    );
    let value = NeoValid::from(
      HeaderValue::from_str(v.as_str()).map_err(|e| ValidationError::new(format!("Parsing failed because of {}", e))),
    );
    name.zip(value)
  })
  .map(|headers| headers.into_iter().collect::<HeaderMap>())
  .trace("responseHeaders")
  .trace("@server")
  .trace("schema")
}

fn configure_server(config_config: &config::Server) -> NeoValid<Server, String> {
  handle_graphiql(config_config.enable_graphiql())
    .zip(validate_hostname(config_config.get_hostname().to_lowercase()))
    .zip(handle_response_headers(config_config.get_response_headers().0))
    .map(|((enable_graphiql, hostname), response_headers)| Server {
      enable_apollo_tracing: config_config.enable_apollo_tracing(),
      enable_cache_control_header: config_config.enable_cache_control(),
      enable_graphiql,
      enable_introspection: config_config.enable_introspection(),
      enable_query_validation: config_config.enable_query_validation(),
      enable_response_validation: config_config.enable_http_validation(),
      global_response_timeout: config_config.get_global_response_timeout(),
      port: config_config.get_port(),
      hostname,
      vars: config_config.get_vars(),
      response_headers,
    })
}
