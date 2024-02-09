use anyhow::{bail, Result};
use hyper::{HeaderMap, Method};
use reqwest::Request;
use url::Url;

use super::protobuf::ProtobufOperation;
use crate::http::Response;
use crate::target_runtime::TargetRuntime;

pub fn create_grpc_request(url: Url, headers: HeaderMap, body: Vec<u8>) -> Request {
    let mut req = Request::new(Method::POST, url);
    req.headers_mut().extend(headers.clone());
    req.body_mut().replace(body.into());

    req
}

pub async fn execute_grpc_request(
    runtime: &TargetRuntime,
    operation: &ProtobufOperation,
    request: Request,
) -> Result<Response<async_graphql::Value>> {
    let response = runtime.http2_only.execute(request).await?;

    if response.status.is_success() {
        return response.to_grpc_value(operation);
    }

    bail!("Failed to execute request")
}
