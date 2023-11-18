use std::collections::{BTreeMap, BTreeSet, HashSet};

use anyhow::Result;
use async_graphql::parser::types::ServiceDocument;
use derive_setters::Setters;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Server, Upstream};
use crate::config::reader::ConfigReader;
use crate::config::source::Source;
use crate::config::{is_default, KeyValues};
use crate::http::Method;
use crate::json::JsonSchema;
use crate::valid::Valid;

#[derive(Serialize, Deserialize, Clone, Debug, Default, Setters)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  #[serde(default)]
  pub server: Server,
  #[serde(default)]
  pub upstream: Upstream,
  pub schema: RootSchema,
  #[serde(default)]
  #[setters(skip)]
  pub types: BTreeMap<String, Type>,
  #[serde(default)]
  pub unions: BTreeMap<String, Union>,
}
impl Config {
  pub fn port(&self) -> u16 {
    self.server.port.unwrap_or(8000)
  }

  pub fn output_types(&self) -> HashSet<&String> {
    let mut types = HashSet::new();

    if let Some(ref query) = &self.schema.query {
      types.insert(query);
    }

    if let Some(ref mutation) = &self.schema.mutation {
      types.insert(mutation);
    }

    for (_, type_of) in self.types.iter() {
      if type_of.interface || !type_of.fields.is_empty() {
        for (_, field) in type_of.fields.iter() {
          types.insert(&field.type_of);
        }
      }
    }
    types
  }

  pub fn input_types(&self) -> HashSet<&String> {
    let mut types = HashSet::new();
    for (_, type_of) in self.types.iter() {
      if !type_of.interface {
        for (_, field) in type_of.fields.iter() {
          for (_, arg) in field.args.iter() {
            types.insert(&arg.type_of);
          }
        }
      }
    }
    types
  }

  pub fn find_type(&self, name: &str) -> Option<&Type> {
    self.types.get(name)
  }

  pub fn find_union(&self, name: &str) -> Option<&Union> {
    self.unions.get(name)
  }

  pub fn to_yaml(&self) -> Result<String> {
    Ok(serde_yaml::to_string(self)?)
  }

  pub fn to_json(&self) -> Result<String> {
    Ok(serde_json::to_string(self)?)
  }

  pub fn to_document(&self) -> ServiceDocument {
    (self.clone()).into()
  }

  pub fn to_sdl(&self) -> String {
    let doc = self.to_document();
    crate::document::print(doc)
  }

  pub fn query(mut self, query: &str) -> Self {
    self.schema.query = Some(query.to_string());
    self
  }

  pub fn types(mut self, types: Vec<(&str, Type)>) -> Self {
    let mut graphql_types = BTreeMap::new();
    for (name, type_) in types {
      graphql_types.insert(name.to_string(), type_);
    }
    self.types = graphql_types;
    self
  }

  pub fn contains(&self, name: &str) -> bool {
    self.types.contains_key(name) || self.unions.contains_key(name)
  }

  pub fn merge_right(self, other: &Self) -> Self {
    let server = self.server.merge_right(other.server.clone());
    let types = merge_types(self.types, other.types.clone());
    let unions = merge_unions(self.unions, other.unions.clone());
    let schema = self.schema.merge_right(other.schema.clone());
    let upstream = self.upstream.merge_right(other.upstream.clone());

    Self { server, upstream, types, schema, unions }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Type {
  pub fields: BTreeMap<String, Field>,
  #[serde(default)]
  pub added_fields: Vec<AddField>,
  pub doc: Option<String>,
  #[serde(default)]
  pub interface: bool,
  #[serde(default)]
  pub implements: BTreeSet<String>,
  #[serde(rename = "enum", default)]
  pub variants: Option<BTreeSet<String>>,
  #[serde(default)]
  pub scalar: bool,
}

impl Type {
  pub fn fields(mut self, fields: Vec<(&str, Field)>) -> Self {
    let mut graphql_fields = BTreeMap::new();
    for (name, field) in fields {
      graphql_fields.insert(name.to_string(), field);
    }
    self.fields = graphql_fields;
    self
  }
  pub fn merge_right(mut self, other: &Self) -> Self {
    let mut fields = self.fields.clone();
    fields.extend(other.fields.clone());
    self.implements.extend(other.implements.clone());
    if let Some(ref variants) = self.variants {
      if let Some(ref other) = other.variants {
        self.variants = Some(variants.union(other).cloned().collect());
      }
    } else {
      self.variants = other.variants.clone();
    }
    Self { fields, ..self.clone() }
  }
}

fn merge_types(mut self_types: BTreeMap<String, Type>, other_types: BTreeMap<String, Type>) -> BTreeMap<String, Type> {
  for (name, mut other_type) in other_types {
    if let Some(self_type) = self_types.remove(&name) {
      other_type = self_type.merge_right(&other_type)
    };

    self_types.insert(name, other_type);
  }
  self_types
}

fn merge_unions(
  mut self_unions: BTreeMap<String, Union>,
  other_unions: BTreeMap<String, Union>,
) -> BTreeMap<String, Union> {
  for (name, mut other_union) in other_unions {
    if let Some(self_union) = self_unions.remove(&name) {
      other_union = self_union.merge_right(other_union);
    }
    self_unions.insert(name, other_union);
  }
  self_unions
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Setters)]
#[setters(strip_option)]
pub struct RootSchema {
  pub query: Option<String>,
  pub mutation: Option<String>,
  pub subscription: Option<String>,
}

impl RootSchema {
  // TODO: add unit-tests
  fn merge_right(self, other: Self) -> Self {
    Self {
      query: other.query.or(self.query),
      mutation: other.mutation.or(self.mutation),
      subscription: other.subscription.or(self.subscription),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Setters)]
#[setters(strip_option)]
pub struct Field {
  #[serde(rename = "type")]
  pub type_of: String,
  #[serde(default)]
  pub list: bool,
  #[serde(default)]
  pub required: bool,
  #[serde(default)]
  pub list_type_required: bool,
  #[serde(default)]
  pub args: BTreeMap<String, Arg>,
  pub doc: Option<String>,
  pub modify: Option<Modify>,
  pub http: Option<Http>,
  #[serde(rename = "unsafe")]
  pub unsafe_operation: Option<Unsafe>,

  #[serde(rename = "const")]
  pub const_field: Option<Const>,
}

impl Field {
  pub fn has_resolver(&self) -> bool {
    self.http.is_some() || self.unsafe_operation.is_some() || self.const_field.is_some()
  }
  pub fn resolvable_directives(&self) -> Vec<&str> {
    let mut directives = Vec::with_capacity(3);
    if self.http.is_some() {
      directives.push("@http")
    }
    if self.unsafe_operation.is_some() {
      directives.push("@unsafe")
    }
    if self.const_field.is_some() {
      directives.push("@const")
    }
    directives
  }
  pub fn has_batched_resolver(&self) -> bool {
    self.http.as_ref().is_some_and(|http| !http.group_by.is_empty())
  }
  pub fn to_list(mut self) -> Self {
    self.list = true;
    self
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Unsafe {
  pub script: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Modify {
  pub name: Option<String>,
  #[serde(default)]
  #[serde(skip_serializing_if = "is_default")]
  pub omit: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Inline {
  pub path: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Arg {
  pub type_of: String,
  #[serde(default)]
  pub list: bool,
  #[serde(default)]
  pub required: bool,
  pub doc: Option<String>,
  pub modify: Option<Modify>,
  pub default_value: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Union {
  pub types: BTreeSet<String>,
  pub doc: Option<String>,
}

impl Union {
  pub fn merge_right(mut self, other: Self) -> Self {
    self.types.extend(other.types);
    self
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Http {
  pub path: String,
  #[serde(default)]
  #[serde(skip_serializing_if = "is_default")]
  pub method: Method,
  #[serde(default)]
  #[serde(skip_serializing_if = "is_default")]
  pub query: KeyValues,
  pub input: Option<JsonSchema>,
  pub output: Option<JsonSchema>,
  pub body: Option<String>,
  #[serde(rename = "baseURL")]
  pub base_url: Option<String>,
  #[serde(default)]
  #[serde(skip_serializing_if = "is_default")]
  pub headers: KeyValues,
  #[serde(default)]
  #[serde(rename = "groupBy", skip_serializing_if = "is_default")]
  pub group_by: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Const {
  pub data: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddField {
  pub name: String,
  pub path: Vec<String>,
}

impl Config {
  pub fn from_json(json: &str) -> Result<Self> {
    Ok(serde_json::from_str(json)?)
  }

  pub fn from_yaml(yaml: &str) -> Result<Self> {
    Ok(serde_yaml::from_str(yaml)?)
  }

  pub fn from_sdl(sdl: &str) -> Valid<Self, String> {
    let doc = async_graphql::parser::parse_schema(sdl);
    match doc {
      Ok(doc) => Valid::from(Config::try_from(doc)),
      Err(e) => Valid::fail(e.to_string()),
    }
  }

  pub fn from_source(source: Source, schema: &str) -> Result<Self> {
    match source {
      Source::GraphQL => Ok(Config::from_sdl(schema).to_result()?),
      Source::Json => Ok(Config::from_json(schema)?),
      Source::Yml => Ok(Config::from_yaml(schema)?),
    }
  }
  pub fn n_plus_one(&self) -> Vec<Vec<(String, String)>> {
    super::n_plus_one::n_plus_one(self)
  }

  pub async fn from_file_or_url(file_paths: std::slice::Iter<'_, String>) -> Result<Config> {
    let config_reader = ConfigReader::init(file_paths);
    config_reader.read().await
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_field_has_or_not_batch_resolver() {
    let f1 = Field { ..Default::default() };

    let f2 =
      Field { http: Some(Http { group_by: vec!["id".to_string()], ..Default::default() }), ..Default::default() };

    let f3 = Field { http: Some(Http { group_by: vec![], ..Default::default() }), ..Default::default() };

    assert!(!f1.has_batched_resolver());
    assert!(f2.has_batched_resolver());
    assert!(!f3.has_batched_resolver());
  }
}
