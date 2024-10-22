use std::borrow::BorrowMut;
use std::sync::Arc;

use derive_setters::Setters;
use serde::Serialize;

use super::server_error::ServerError;
use super::Positioned;
use crate::core::async_graphql_hyper::CacheControl;
use crate::core::jit;

#[derive(Clone, Setters, Serialize, Debug)]
pub struct Response<Value> {
    #[serde(default)]
    pub data: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ServerError>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<(String, Value)>,

    #[serde(skip)]
    pub cache_control: CacheControl,
}

impl<Value: Default> Response<Value> {
    pub fn new(result: Result<Value, Positioned<jit::Error>>) -> Self {
        match result {
            Ok(value) => Response {
                data: value,
                errors: Vec::new(),
                extensions: Vec::new(),
                cache_control: Default::default(),
            },
            Err(error) => Response {
                data: Default::default(),
                errors: vec![error.into()],
                extensions: Vec::new(),
                cache_control: Default::default(),
            },
        }
    }

    pub fn add_errors(&mut self, new_errors: Vec<Positioned<jit::Error>>) {
        self.errors.extend(new_errors.into_iter().map(|e| e.into()));
    }
}

impl Response<async_graphql::Value> {
    pub fn merge_with(mut self, other: async_graphql::Response) -> Self {
        if let async_graphql::Value::Object(mut other_obj) = other.data {
            if let async_graphql::Value::Object(self_obj) = std::mem::take(self.data.borrow_mut()) {
                other_obj.extend(self_obj);
                self.data = async_graphql::Value::Object(other_obj);
            } else {
                self.data = async_graphql::Value::Object(other_obj);
            }
        }

        self.errors
            .extend(other.errors.into_iter().map(|e| e.into()));
        self.extensions.extend(other.extensions);

        self
    }
}

/// Represents a GraphQL response in a serialized byte format.
#[derive(Clone)]
pub struct AnyResponse<Body> {
    /// The GraphQL response data serialized into a byte array.
    pub body: Arc<Body>,

    /// Information regarding cache policies for the response, such as max age
    /// and public/private settings.
    pub cache_control: CacheControl,

    /// Indicates whether graphql response contains error or not.
    pub is_ok: bool,
}

impl<Body> Default for AnyResponse<Body>
where
    Body: Default,
{
    fn default() -> Self {
        Self {
            body: Default::default(),
            cache_control: Default::default(),
            is_ok: true,
        }
    }
}

impl From<Response<async_graphql::Value>> for AnyResponse<Vec<u8>> {
    fn from(response: Response<async_graphql::Value>) -> Self {
        Self {
            cache_control: CacheControl {
                max_age: response.cache_control.max_age,
                public: response.cache_control.public,
            },
            is_ok: response.errors.is_empty(),
            // Safely serialize the response to JSON bytes. Since the response is always valid,
            // serialization is expected to succeed. In the unlikely event of a failure,
            // default to an empty byte array. TODO: return error instead of default
            // value.
            body: Arc::new(serde_json::to_vec(&response).unwrap_or_default()),
        }
    }
}

pub enum BatchResponse<Body> {
    Single(AnyResponse<Body>),
    Batch(Vec<AnyResponse<Body>>),
}

impl<Body> BatchResponse<Body> {
    pub fn is_ok(&self) -> bool {
        match self {
            BatchResponse::Single(s) => s.is_ok,
            BatchResponse::Batch(b) => b.iter().all(|s| s.is_ok),
        }
    }

    /// Modifies the cache control values with the provided one.
    pub fn cache_control(&self, cache_control: Option<&CacheControl>) -> CacheControl {
        match self {
            BatchResponse::Single(resp) => cache_control.unwrap_or(&resp.cache_control).clone(),
            BatchResponse::Batch(responses) => {
                responses.iter().fold(CacheControl::default(), |acc, resp| {
                    if let Some(cc) = cache_control {
                        acc.merge(cc)
                    } else {
                        acc.merge(&resp.cache_control)
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use async_graphql_value::ConstValue;

    use super::Response;
    use crate::core::jit::{self, Pos, Positioned};
    use crate::core::merge_right::MergeRight;

    #[test]
    fn test_with_response() {
        let value = ConstValue::String("Tailcall - Modern GraphQL Runtime".into());
        let response = Response::<ConstValue>::new(Ok(value.clone()));

        assert_eq!(response.data, value);
        assert!(response.errors.is_empty());
        assert!(response.extensions.is_empty());
    }

    #[test]
    fn test_with_error() {
        let error = Positioned::new(
            jit::Error::Validation(jit::ValidationError::ValueRequired),
            Pos { line: 1, column: 2 },
        );
        let response = Response::<ConstValue>::new(Err(error.clone()));

        assert!(response.extensions.is_empty());

        assert_eq!(response.errors.len(), 1);
        insta::assert_debug_snapshot!(response);
    }

    #[test]
    fn test_adding_errors() {
        let value = ConstValue::String("Tailcall - Modern GraphQL Runtime".into());
        let mut response = Response::<ConstValue>::new(Ok(value.clone()));

        // Initially no errors
        assert!(response.errors.is_empty());

        // Add an error
        let error = Positioned::new(
            jit::Error::Validation(jit::ValidationError::ValueRequired),
            Pos { line: 1, column: 2 },
        );
        response.add_errors(vec![error.clone()]);

        assert_eq!(response.errors.len(), 1);
        insta::assert_debug_snapshot!(response);
    }

    #[test]
    fn test_conversion_to_async_graphql() {
        let error1 = Positioned::new(
            jit::Error::Validation(jit::ValidationError::ValueRequired),
            Pos { line: 1, column: 2 },
        );
        let error2 = Positioned::new(
            jit::Error::Validation(jit::ValidationError::EnumInvalid {
                type_of: "EnumDef".to_string(),
            }),
            Pos { line: 3, column: 4 },
        );

        let mut response = Response::<ConstValue>::new(Ok(ConstValue::Null));
        response.add_errors(vec![error2, error1]);

        let async_response = response;

        assert_eq!(async_response.errors.len(), 2);
        insta::assert_debug_snapshot!(async_response);
    }

    #[test]
    pub fn test_merging_of_responses() {
        let introspection_response = r#"
        {
            "__type": {
                "name": "User",
                "fields": [
                    {
                        "name": "birthday",
                        "type": {
                            "name": "Date"
                        }
                    },
                    {
                        "name": "id",
                        "type": {
                            "name": "String"
                        }
                    }
                ]
            }
        }
        "#;
        let introspection_data =
            ConstValue::from_json(serde_json::from_str(introspection_response).unwrap()).unwrap();
        let introspection_response = async_graphql::Response::new(introspection_data);

        let user_response = r#"
        {
            "me": {
                "id": 1,
                "name": "John Smith",
                "birthday": "2023-03-08T12:45:26-05:00"
            }
        }
        "#;
        let user_data =
            ConstValue::from_json(serde_json::from_str(user_response).unwrap()).unwrap();
        let query_response = async_graphql::Response::new(user_data);

        let merged_response = introspection_response.merge_right(query_response);

        insta::assert_json_snapshot!(merged_response);
    }

    #[test]
    pub fn test_merging_of_errors() {
        let mut resp1 = async_graphql::Response::new(ConstValue::default());
        let mut err1 = vec![async_graphql::ServerError::new("Error-1", None)];
        resp1.errors.append(&mut err1);

        let mut resp2 = async_graphql::Response::new(ConstValue::default());
        let mut err2 = vec![async_graphql::ServerError::new(
            "Error-2",
            Some(async_graphql::Pos::default()),
        )];
        resp2.errors.append(&mut err2);

        let merged_resp = resp1.merge_right(resp2);
        insta::assert_json_snapshot!(merged_resp);
    }
}
