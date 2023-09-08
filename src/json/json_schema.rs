use crate::valid::{Valid, ValidExtensions, VectorExtension};
use async_graphql::Name;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "schema")]
pub enum JsonSchema {
    Obj(HashMap<String, JsonSchema>),
    Arr(Box<JsonSchema>),
    Opt(Box<JsonSchema>),
    Str,
    Num,
    Bool,
}

impl<const L: usize> From<[(&'static str, JsonSchema); L]> for JsonSchema {
    fn from(fields: [(&'static str, JsonSchema); L]) -> Self {
        let mut map = HashMap::new();
        for (name, schema) in fields {
            map.insert(name.to_string(), schema);
        }
        JsonSchema::Obj(map)
    }
}

impl Default for JsonSchema {
    fn default() -> Self {
        JsonSchema::Obj(HashMap::new())
    }
}

impl JsonSchema {
    // TODO: validate `JsonLike` instead of fixing on `async_graphql::Value`
    pub fn validate(&self, value: &async_graphql::Value) -> Valid<(), &'static str> {
        match self {
            JsonSchema::Str => match value {
                async_graphql::Value::String(_) => Valid::Ok(()),
                _ => Valid::fail("expected string"),
            },
            JsonSchema::Num => match value {
                async_graphql::Value::Number(_) => Valid::Ok(()),
                _ => Valid::fail("expected number"),
            },
            JsonSchema::Bool => match value {
                async_graphql::Value::Boolean(_) => Valid::Ok(()),
                _ => Valid::fail("expected boolean"),
            },
            JsonSchema::Arr(schema) => match value {
                async_graphql::Value::List(list) => {
                    // TODO: add unit tests
                    list.iter()
                        .enumerate()
                        .validate_all(|(i, item)| schema.validate(item).trace(i.to_string().as_str()))?;
                    Ok(())
                }
                _ => Valid::fail("expected array"),
            },
            JsonSchema::Obj(fields) => {
                let field_schema_list: Vec<(&String, &JsonSchema)> = fields.iter().collect();
                match value {
                    async_graphql::Value::Object(value_map) => {
                        let items_valid = field_schema_list.validate_all(|(name, field_schema)| {
                            if let Some(field_value) = value_map.get(&Name::new(name)) {
                                field_schema.validate(field_value).trace(name)
                            } else {
                                Valid::fail("expected field")
                            }
                        });
                        match items_valid {
                            Valid::Ok(_) => Valid::Ok(()),
                            Valid::Err(err) => Valid::Err(err),
                        }
                    }
                    _ => Valid::fail("expected object"),
                }
            }
            JsonSchema::Opt(schema) => match value {
                async_graphql::Value::Null => Valid::Ok(()),
                _ => schema.validate(value),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::valid::Valid;
    use crate::{json::JsonSchema, valid::ValidExtensions};
    use async_graphql::Name;
    use indexmap::IndexMap;

    #[test]
    fn test_validate_string() {
        let schema = JsonSchema::Str;
        let value = async_graphql::Value::String("hello".to_string());
        let result = schema.validate(&value);
        assert_eq!(result, Valid::Ok(()));
    }

    #[test]
    fn test_validate_valid_object() {
        let schema = JsonSchema::from([("name", JsonSchema::Str), ("age", JsonSchema::Num)]);
        let value = async_graphql::Value::Object({
            let mut map = IndexMap::new();
            map.insert(Name::new("name"), async_graphql::Value::String("hello".to_string()));
            map.insert(Name::new("age"), async_graphql::Value::Number(1.into()));
            map
        });
        let result = schema.validate(&value);
        assert_eq!(result, Valid::Ok(()));
    }

    #[test]
    fn test_validate_invalid_object() {
        let schema = JsonSchema::from([("name", JsonSchema::Str), ("age", JsonSchema::Num)]);
        let value = async_graphql::Value::Object({
            let mut map = IndexMap::new();
            map.insert(Name::new("name"), async_graphql::Value::String("hello".to_string()));
            map.insert(Name::new("age"), async_graphql::Value::String("1".to_string()));
            map
        });
        let result = schema.validate(&value);
        assert_eq!(result, Valid::fail("expected number").trace("age"));
    }
}
