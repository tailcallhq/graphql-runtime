use schemars::JsonSchema;
use tailcall_macros::ScalarDefinition;

use crate::core::json::JsonLike;

/// A field whose value conforms to the standard URL format as specified in RFC3986 (https://www.ietf.org/rfc/rfc3986.txt), and it uses real JavaScript URL objects.
#[derive(JsonSchema, Default, ScalarDefinition, Clone, Debug)]
pub struct Url {
    #[allow(dead_code)]
    #[serde(rename = "Url")]
    pub url: String,
}

impl super::Scalar for Url {
    /// Function used to validate the date
    fn validate<'a, Value: JsonLike<'a>>(&self) -> fn(&'a Value) -> bool {
        |value| {
            if let Some(date_str) = value.as_str() {
                return url::Url::parse(date_str).is_ok();
            }
            false
        }
    }
}

#[cfg(test)]
mod test {
    use async_graphql_value::ConstValue;

    use super::*;
    use crate::core::scalar::Scalar;

    #[test]
    fn test_url() {
        let date = Url::default();
        let validate = date.validate()(&ConstValue::String("https://ssdd.dev".to_string()));
        assert!(validate);
    }

    #[test]
    fn test_invalid_url() {
        let date = Url::default();
        let validate = date.validate()(&ConstValue::String("localhost".to_string()));
        assert!(!validate);
    }

    #[test]
    fn test_invalid_value() {
        let date = Url::default();
        let validate = date.validate()(&ConstValue::Null);
        assert!(!validate);
    }
}
