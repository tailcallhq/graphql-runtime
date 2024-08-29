use std::collections::HashMap;

use async_graphql_value::ConstValue;
use serde::Deserialize;

use super::{Builder, OperationPlan, Result, Variables};
use crate::core::blueprint::Blueprint;

#[derive(Debug, Deserialize, Clone)]
pub struct Request<V> {
    #[serde(default)]
    pub query: String,
    #[serde(default, rename = "operationName")]
    pub operation_name: Option<String>,
    #[serde(default)]
    pub variables: Variables<V>,
    #[serde(default)]
    pub extensions: HashMap<String, V>,
}

impl From<&async_graphql::Request> for Request<ConstValue> {
    fn from(value: &async_graphql::Request) -> Self {
        Self {
            query: value.query.clone(),
            operation_name: value.operation_name.clone(),
            variables: Variables::from_iter(
                value
                    .variables
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_owned())),
            ),
            extensions: value.extensions.clone(),
        }
    }
}

impl Request<ConstValue> {
    pub fn create_plan(&self, blueprint: &Blueprint) -> Result<OperationPlan<ConstValue>> {
        let doc = async_graphql::parser::parse_query(&self.query)?;
        let builder = Builder::new(blueprint, doc);
        let plan = builder.build(&self.variables, self.operation_name.as_deref())?;

        Ok(plan)
    }
}

impl<V> Request<V> {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            operation_name: None,
            variables: Variables::new(),
            extensions: HashMap::new(),
        }
    }

    pub fn variables(self, vars: impl IntoIterator<Item = (String, V)>) -> Self {
        Self { variables: Variables::from_iter(vars), ..self }
    }
}
