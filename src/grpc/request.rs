use anyhow::{bail, Result};
use hyper::{HeaderMap, Method};
use reqwest::Request;
use url::Url;

use super::protobuf::ProtobufOperation;
use crate::http::{HttpClient, Response};

pub fn create_grpc_request(url: Url, headers: HeaderMap, body: Vec<u8>) -> Request {
  let mut req = Request::new(Method::POST, url);
  #[cfg(feature = "default")]
  super::set_req_version(&mut req);
  req.headers_mut().extend(headers.clone());
  req.body_mut().replace(body.into());

  req
}

pub async fn execute_grpc_request(
  client: &dyn HttpClient,
  operation: &ProtobufOperation,
  request: Request,
) -> Result<Response<async_graphql::Value>> {
  let response = client.execute_raw(request).await?;

  if response.status.is_success() {
    return response.to_value(Some(operation));
  }

  bail!("Failed to execute request")
}
