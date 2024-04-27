extern crate core;

use std::panic;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use hyper::body::Bytes;
use reqwest::header::{HeaderName, HeaderValue};
use serde_json::Value;
use tailcall::http::Response;
use tailcall::HttpIO;

use super::runtime::{ExecutionMock, ExecutionSpec};

#[derive(Clone, Debug)]
pub struct Http {
    mocks: Vec<ExecutionMock>,
    spec_path: String,
}

impl Http {
    pub fn new(spec: &ExecutionSpec) -> Self {
        let mocks = spec
            .mock
            .as_ref()
            .map(|mocks| {
                mocks
                    .iter()
                    .map(|mock| ExecutionMock {
                        mock: mock.clone(),
                        actual_hits: Arc::new(AtomicUsize::default()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let spec_path = spec
            .path
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap_or(&spec.path)
            .to_string_lossy()
            .into_owned();

        Http { mocks, spec_path }
    }

    pub fn test_hits(&self, path: impl AsRef<Path>) {
        for mock in &self.mocks {
            mock.test_hits(path.as_ref());
        }
    }
}

fn string_to_bytes(input: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('0') => bytes.push(0),
                Some('n') => bytes.push(b'\n'),
                Some('t') => bytes.push(b'\t'),
                Some('r') => bytes.push(b'\r'),
                Some('\\') => bytes.push(b'\\'),
                Some('\"') => bytes.push(b'\"'),
                Some('x') => {
                    let mut hex = chars.next().unwrap().to_string();
                    hex.push(chars.next().unwrap());
                    let byte = u8::from_str_radix(&hex, 16).unwrap();
                    bytes.push(byte);
                }
                _ => panic!("Unsupported escape sequence"),
            },
            _ => bytes.push(c as u8),
        }
    }

    bytes
}

#[async_trait::async_trait]
impl HttpIO for Http {
    async fn execute(&self, req: reqwest::Request) -> anyhow::Result<Response<Bytes>> {
        // Determine if the request is a GRPC request based on PORT
        let is_grpc = req.url().as_str().contains("50051");

        // Try to find a matching mock for the incoming request.
        let execution_mock = self
            .mocks
            .iter()
            .find(|mock| {
                let mock_req = &mock.mock.request;
                let method_match = req.method() == mock_req.0.method.clone().to_hyper();
                let url_match = req.url().as_str() == mock_req.0.url.clone().as_str();
                let req_body = match req.body() {
                    Some(body) => {
                        if let Some(bytes) = body.as_bytes() {
                            if let Ok(body_str) = std::str::from_utf8(bytes) {
                                Value::from(body_str)
                            } else {
                                Value::Null
                            }
                        } else {
                            Value::Null
                        }
                    }
                    None => Value::Null,
                };
                let body_match = req_body == mock_req.0.body;
                let headers_match = req
                    .headers()
                    .iter()
                    .filter(|(key, _)| *key != "content-type")
                    .all(|(key, value)| {
                        let header_name = key.to_string();

                        let header_value = value.to_str().unwrap();
                        let mock_header_value = "".to_string();
                        let mock_header_value = mock_req
                            .0
                            .headers
                            .get(&header_name)
                            .unwrap_or(&mock_header_value);
                        header_value == mock_header_value
                    });
                method_match && url_match && headers_match && (body_match || is_grpc)
            })
            .ok_or(anyhow!(
                "No mock found for request: {:?} {} in {}",
                req.method(),
                req.url(),
                self.spec_path
            ))?;

        execution_mock.actual_hits.fetch_add(1, Ordering::Relaxed);

        // Clone the response from the mock to avoid borrowing issues.
        let mock_response = execution_mock.mock.response.clone();

        // Build the response with the status code from the mock.
        let status_code = reqwest::StatusCode::from_u16(mock_response.0.status)?;

        if status_code.is_client_error() || status_code.is_server_error() {
            return Err(anyhow::format_err!("Status code error"));
        }

        let mut response = Response { status: status_code, ..Default::default() };

        // Insert headers from the mock into the response.
        for (key, value) in mock_response.0.headers {
            let header_name = HeaderName::from_str(&key)?;
            let header_value = HeaderValue::from_str(&value)?;
            response.headers.insert(header_name, header_value);
        }

        // Special Handling for GRPC
        if let Some(body) = mock_response.0.text_body {
            // Return plaintext body if specified
            let body = string_to_bytes(&body);
            response.body = Bytes::from_iter(body);
        } else if is_grpc {
            // Special Handling for GRPC
            let body = string_to_bytes(mock_response.0.body.as_str().unwrap_or_default());
            response.body = Bytes::from_iter(body);
        } else {
            let body = serde_json::to_vec(&mock_response.0.body)?;
            response.body = Bytes::from_iter(body);
        }

        Ok(response)
    }
}
