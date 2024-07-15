use std::collections::HashMap;

use derive_setters::Setters;
use serde::Deserialize;

use super::{Builder, Error, ExecutionPlan, Result};
use crate::core::blueprint::Blueprint;
use crate::core::jit::model::Variables;

#[derive(Debug, Deserialize, Setters, Clone)]
pub struct Request<Value> {
    #[serde(default)]
    pub query: String,
    #[serde(default, rename = "operationName")]
    pub operation_name: Option<String>,
    #[serde(default)]
    pub variables: Variables<Value>,
    #[serde(default)]
    pub extensions: HashMap<String, Value>,
}

impl From<async_graphql::Request> for Request<async_graphql_value::ConstValue> {
    fn from(value: async_graphql::Request) -> Self {
        Self {
            query: value.query,
            operation_name: value.operation_name,
            variables: match value.variables.into_value() {
                async_graphql_value::ConstValue::Object(val) => {
                    Variables::from_iter(val.into_iter().map(|(name, val)| (name.to_string(), val)))
                }
                _ => Variables::default(),
            },
            extensions: value.extensions,
        }
    }
}

impl<Value> Request<Value> {
    pub fn try_new(&self, blueprint: &Blueprint) -> Result<ExecutionPlan> {
        let doc = async_graphql::parser::parse_query(&self.query)?;
        let builder = Builder::new(blueprint, doc);
        builder.build().map_err(Error::BuildError)
    }
}

impl<A> Request<A> {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            operation_name: None,
            variables: Variables::default(),
            extensions: HashMap::new(),
        }
    }
}
