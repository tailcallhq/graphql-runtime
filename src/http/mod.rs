mod client;
mod data_loader;

mod data_loader_request;
mod http_1;
mod http_2;
mod method;
mod request_context;
mod request_handler;
mod response;
mod server;
mod server_config;
mod server_context;

use std::time::Duration;

use cache_control::CacheControl;
pub use client::*;
pub use data_loader::*;
pub use data_loader_request::*;
use hyper::header::CACHE_CONTROL;
pub use method::Method;
pub use request_context::RequestContext;
pub use request_handler::{handle_batch_request, handle_single_request};
pub use response::*;
pub use server::start_server;
pub use server_context::ServerContext;

use self::server_config::ServerConfig;

pub fn max_age(res: &Response) -> Option<Duration> {
  let header = res.headers.get(CACHE_CONTROL)?;
  let value = header.to_str().ok()?;
  let policy = CacheControl::from_value(value)?;
  policy.max_age
}

/// Returns the minimum TTL of the given responses.
pub fn min_ttl<'a>(res_vec: impl Iterator<Item = &'a Response>) -> i32 {
  let mut min = -1;

  for res in res_vec {
    if let Some(max_age) = max_age(res) {
      let ttl = max_age.as_secs() as i32;
      if min == -1 || ttl < min {
        min = ttl;
      }
    }
  }
  min
}

fn log_launch(sc: &ServerConfig) {
  let addr = sc.addr().to_string();
  log::info!("🚀 Tailcall launched at [{}] over {}", addr, sc.http_version());
  if sc.graphiql() {
    log::info!("🌍 Playground: https://{}", addr);
  }
}

#[cfg(test)]
mod tests {

  use std::time::Duration;

  use hyper::HeaderMap;

  use crate::http::Response;

  fn cache_control_header(i: i32) -> HeaderMap {
    let mut headers = reqwest::header::HeaderMap::default();
    headers.append("Cache-Control", format!("max-age={}", i).parse().unwrap());
    headers
  }

  #[test]
  fn test_max_age_none() {
    let response = Response::default();
    assert_eq!(super::max_age(&response), None);
  }

  #[test]
  fn test_max_age_some() {
    let headers = cache_control_header(3600);
    let response = Response::default().headers(headers);

    assert_eq!(super::max_age(&response), Some(Duration::from_secs(3600)));
  }

  #[test]
  fn test_min_ttl() {
    let max_ages = [3600, 1800, 7200].map(|i| Response::default().headers(cache_control_header(i)));
    let min = super::min_ttl(max_ages.iter());
    assert_eq!(min, 1800);
  }
}
