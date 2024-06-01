use anyhow::Result;
use async_graphql_value::{ConstValue, Name};
use derive_setters::Setters;
use hyper::body::Bytes;
use indexmap::IndexMap;
use prost::Message;
use tonic::Status;
use tonic_types::Status as GrpcStatus;
use crate::core::{extend_lifetime, FromValue};

use crate::core::grpc::protobuf::ProtobufOperation;
use crate::core::ir::EvaluationError;

#[derive(Clone, Debug, Default, Setters)]
pub struct Response<Body> {
    pub status: reqwest::StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: Body,
}

// Trait to convert a serde_json_borrow::Value to a ConstValue.
// serde_json_borrow::Value is a borrowed version of serde_json::Value.
// It has a limited lifetime tied to the input JSON, making it more
// efficient. Benchmarking is required to determine the performance If any
// change is made.

impl Response<Bytes> {
    pub async fn from_reqwest(resp: reqwest::Response) -> Result<Self> {
        let status = resp.status();
        let headers = resp.headers().to_owned();
        let body = resp.bytes().await?;
        Ok(Response { status, headers, body })
    }

    pub fn empty() -> Self {
        Response {
            status: reqwest::StatusCode::OK,
            headers: reqwest::header::HeaderMap::default(),
            body: Bytes::new(),
        }
    }

    pub fn to_json<T: Default + FromValue>(self) -> Result<Response<T>> {
        if self.body.is_empty() {
            return Ok(Response {
                status: self.status,
                headers: self.headers,
                body: Default::default(),
            });
        }
        // Note: We convert the body to a serde_json_borrow::Value for better
        // performance. Warning: Do not change this to direct conversion to `T`
        // without benchmarking the performance impact.
        let body: serde_json_borrow::Value = serde_json::from_slice(&self.body)?;
        let body = T::from_value(body);
        Ok(Response { status: self.status, headers: self.headers, body })
    }

    pub fn into_borrowed_json(self) -> Result<Response<crate::core::ConstValue>> {
        if self.body.is_empty() {
            return Ok(Response {
                status: self.status,
                headers: self.headers,
                body: crate::core::ConstValue::Null,
            });
        }
        let body = extend_lifetime(serde_json::from_slice::<serde_json_borrow::Value>(self.body.to_vec().as_slice())?);
        Ok(Response { status: self.status, headers: self.headers, body })
    }

    pub fn to_grpc_value(
        self,
        operation: &ProtobufOperation,
    ) -> Result<Response<crate::core::ConstValue>> {
        let mut resp = Response::default();
        let body = operation.convert_output::<serde_json::Value>(&self.body)?;
        let body = crate::core::ConstValue::from(body);
        resp.body = body;
        resp.status = self.status;
        resp.headers = self.headers;
        Ok(resp)
    }

    pub fn to_grpc_error(&self, operation: &ProtobufOperation) -> anyhow::Error {
        let grpc_status = match Status::from_header_map(&self.headers) {
            Some(status) => status,
            None => {
                return EvaluationError::IOException(
                    "Error while parsing upstream headers".to_owned(),
                )
                .into()
            }
        };

        let mut obj: IndexMap<Name, async_graphql::Value> = IndexMap::new();
        let mut status_details = Vec::new();
        if !grpc_status.details().is_empty() {
            if let Ok(status) = GrpcStatus::decode(grpc_status.details()) {
                obj.insert(Name::new("code"), status.code.into());
                obj.insert(Name::new("message"), status.message.clone().into());

                for detail in status.details {
                    let type_url = &detail.type_url;
                    let type_name = type_url.split('/').last().unwrap_or("");

                    if let Some(message) = operation.find_message(type_name) {
                        if let Ok(decoded) = message.decode(detail.value.as_slice()) {
                            status_details.push(decoded);
                        } else {
                            tracing::error!("Error while decoding message: {type_name}");
                        }
                    } else {
                        tracing::error!(
                            "Error while searching descriptor for message: {type_name}"
                        );
                    }
                }
            } else {
                tracing::error!("Error while decoding gRPC status details");
            }
        }
        obj.insert(Name::new("details"), ConstValue::List(status_details));

        let error = EvaluationError::GRPCError {
            grpc_code: grpc_status.code() as i32,
            grpc_description: grpc_status.code().description().to_owned(),
            grpc_status_message: grpc_status.message().to_owned(),
            grpc_status_details: ConstValue::Object(obj),
        };

        // TODO: because of this conversion to anyhow::Error
        // we lose additional details that could be added
        // through async_graphql::ErrorExtensions
        anyhow::Error::new(error)
    }

    pub fn to_resp_string(self) -> Result<Response<String>> {
        Ok(Response::<String> {
            body: String::from_utf8(self.body.to_vec())?,
            status: self.status,
            headers: self.headers,
        })
    }
}
