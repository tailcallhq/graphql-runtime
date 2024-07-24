use async_graphql::validators::email;
use schemars::JsonSchema;
use tailcall_macros::ScalarDefinition;

use crate::core::json::JsonLike;

/// field whose value conforms to the standard internet email address format as specified in HTML Spec: https://html.spec.whatwg.org/multipage/input.html#valid-e-mail-address.
#[derive(JsonSchema, Default, ScalarDefinition, Clone, Debug)]
pub struct Email {
    #[allow(dead_code)]
    #[serde(rename = "Email")]
    #[schemars(schema_with = "email_schema")]
    pub email: String,
}

fn email_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    let mut schema: schemars::schema::SchemaObject = <String>::json_schema(gen).into();
    schema.string = Some(Box::new(schemars::schema::StringValidation {
        pattern: Some("/^[a-zA-Z0-9.!#$%&'*+\\/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/".to_owned()),
        ..Default::default()
    }));
    schema.into()
}

impl super::Scalar for Email {
    /// Function used to validate the email address
    fn validate<'a, Value: JsonLike<'a>>(&self) -> fn(&'a Value) -> bool {
        |value| {
            if let Some(email_str) = value.as_str() {
                let email_str = email_str.to_string();
                return email(&email_str).is_ok();
            }
            false
        }
    }
}

#[cfg(test)]
mod test {
    use async_graphql_value::ConstValue;

    use crate::core::scalar::{Email, Scalar};

    #[test]
    fn test_email_valid_req_resp() {
        assert!(Email::default().validate()(&ConstValue::String(
            "valid@email.com".to_string()
        )));
    }

    #[test]
    fn test_email_invalid() {
        assert!(!Email::default().validate()(&ConstValue::String(
            "invalid_email".to_string()
        )));
    }

    #[test]
    fn test_email_invalid_const_value() {
        assert!(!Email::default().validate()(&ConstValue::Null));
    }
}
