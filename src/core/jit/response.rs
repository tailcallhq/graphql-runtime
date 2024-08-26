use derive_setters::Setters;
use serde::Serialize;

use super::Positioned;
use crate::core::jit;

#[derive(Setters, Serialize)]
pub struct Response<Value, Error> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<Positioned<Error>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<(String, Value)>,
}

impl<Value, Error> Response<Value, Error> {
    pub fn new(result: Result<Value, Positioned<Error>>) -> Self {
        match result {
            Ok(value) => Response {
                data: Some(value),
                errors: Vec::new(),
                extensions: Vec::new(),
            },
            Err(error) => Response { data: None, errors: vec![error], extensions: Vec::new() },
        }
    }

    pub fn add_errors(&mut self, new_errors: Vec<Positioned<Error>>) {
        self.errors.extend(new_errors);
    }
}

impl Response<async_graphql::Value, jit::Error> {
    pub fn into_async_graphql(self) -> async_graphql::Response {
        let mut resp = async_graphql::Response::new(self.data.unwrap_or_default());
        for (name, value) in self.extensions {
            resp = resp.extension(name, value);
        }
        for error in self.errors {
            resp.errors.push(error.into());
        }
        resp
    }
}

#[cfg(test)]
mod test {
    use async_graphql_value::ConstValue;

    use super::Response;
    use crate::core::jit::{self, Pos, Positioned};

    #[test]
    fn test_with_response() {
        let value = ConstValue::String("Tailcall - Modern GraphQL Runtime".into());
        let response = Response::<ConstValue, jit::Error>::new(Ok(value.clone()));

        assert!(response.data.is_some());
        assert_eq!(response.data, Some(value));
        assert!(response.errors.is_empty());
        assert!(response.extensions.is_empty());
    }

    #[test]
    fn test_with_error() {
        let error = Positioned::new(
            jit::Error::Validation(jit::ValidationError::ValueRequired),
            Pos { line: 1, column: 2 },
        );
        let response = Response::<ConstValue, jit::Error>::new(Err(error.clone()));

        assert!(response.data.is_none());
        assert!(response.extensions.is_empty());

        assert_eq!(response.errors.len(), 1);
        insta::assert_debug_snapshot!(response.into_async_graphql());
    }

    #[test]
    fn test_adding_errors() {
        let value = ConstValue::String("Tailcall - Modern GraphQL Runtime".into());
        let mut response = Response::<ConstValue, jit::Error>::new(Ok(value.clone()));

        // Initially no errors
        assert!(response.errors.is_empty());

        // Add an error
        let error = Positioned::new(
            jit::Error::Validation(jit::ValidationError::ValueRequired),
            Pos { line: 1, column: 2 },
        );
        response.add_errors(vec![error.clone()]);

        assert_eq!(response.errors.len(), 1);
        insta::assert_debug_snapshot!(response.into_async_graphql());
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

        let mut response = Response::<ConstValue, jit::Error>::new(Ok(ConstValue::Null));
        response.add_errors(vec![error2, error1]);

        let async_response = response.into_async_graphql();

        assert_eq!(async_response.errors.len(), 2);
        insta::assert_debug_snapshot!(async_response);
    }
}
