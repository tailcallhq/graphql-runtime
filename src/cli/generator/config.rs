use std::env;
use std::marker::PhantomData;
use std::path::Path;

use path_clean::PathClean;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::core::config::{self};

#[derive(Serialize, Deserialize, Default, Debug, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config<Status = UnResolved> {
    pub input: Vec<Input<Status>>,
    pub output: Output<Status>,
    #[serde(default)]
    pub generate: GenerateOptions,
    #[serde(default)]
    pub transform: Transform,
    #[serde(skip_serializing, skip_deserializing)]
    _marker: PhantomData<Status>,
}

#[derive(Debug)]
pub enum Resolved {}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub enum UnResolved {}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InputSource<Status = UnResolved> {
    Config {
        src: String,
        #[serde(skip_serializing, skip_deserializing)]
        _marker: PhantomData<Status>,
    },
    Import {
        src: String,
        #[serde(skip_serializing, skip_deserializing)]
        _marker: PhantomData<Status>,
    },
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Input<Status = UnResolved> {
    #[serde(flatten)]
    pub source: InputSource<Status>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output<Status = UnResolved> {
    /// Controls the output format (graphql, json, yaml)
    pub format: config::Source,
    /// Specifies the output file name
    pub file: String,
    #[serde(skip_serializing, skip_deserializing)]
    phantom: PhantomData<Status>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(default = "defaults::schema::query")]
    pub query: String,
    #[serde(default = "defaults::schema::mutation")]
    pub mutation: String,
}

#[derive(Serialize, Deserialize, Default, Debug, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerateOptions {
    #[serde(default)]
    pub schema: Schema,
}

#[derive(Serialize, Deserialize, Default, Debug, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    // TODO: change types
    pub ambiguous_name_resolver: Option<serde_json::Value>,
    pub merge_types: Option<serde_json::Value>,
    pub js: Option<serde_json::Value>,
}

impl InputSource<UnResolved> {
    pub fn resolve(self, parent_dir: Option<&Path>) -> anyhow::Result<InputSource<Resolved>> {
        match self {
            InputSource::Config { src, _marker } => Ok(InputSource::Config {
                src: resolve(src.as_str(), parent_dir)?,
                _marker: PhantomData,
            }),
            InputSource::Import { src, _marker } => Ok(InputSource::Import {
                src: resolve(src.as_str(), parent_dir)?,
                _marker: PhantomData,
            }),
        }
    }
}

impl Input<UnResolved> {
    pub fn resolve(self, parent_dir: Option<&Path>) -> anyhow::Result<Input<Resolved>> {
        let resolved_source = self.source.resolve(parent_dir)?;
        Ok(Input { source: resolved_source })
    }
}

impl Output<UnResolved> {
    pub fn resolve(self, parent_dir: Option<&Path>) -> anyhow::Result<Output<Resolved>> {
        Ok(Output {
            format: self.format,
            file: resolve(&self.file, parent_dir)?,
            phantom: PhantomData,
        })
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            query: defaults::schema::query(),
            mutation: defaults::schema::mutation(),
        }
    }
}

impl Config {
    /// Resolves all the paths present inside the GeneratorConfig.
    pub fn resolve_paths(self, config_path: &str) -> anyhow::Result<Config<Resolved>> {
        let parent_dir = Some(Path::new(config_path).parent().unwrap_or(Path::new("")));

        let resolved_inputs = self
            .input
            .into_iter()
            .map(|input| input.resolve(parent_dir))
            .collect::<anyhow::Result<Vec<Input<_>>>>()?;

        Ok(Config {
            input: resolved_inputs,
            output: self.output.resolve(parent_dir)?,
            generate: self.generate,
            transform: self.transform,
            _marker: PhantomData,
        })
    }
}

mod defaults {
    pub mod schema {
        pub fn query() -> String {
            "Query".to_string()
        }

        pub fn mutation() -> String {
            "Mutation".to_string()
        }
    }
}

// TODO: In our codebase we've similar functions like below, create a separate
// module for helpers functions like these.
fn resolve(path: &str, parent_dir: Option<&Path>) -> anyhow::Result<String> {
    if Url::parse(path).is_ok() || Path::new(path).is_absolute() {
        return Ok(path.to_string());
    }

    let parent_dir = parent_dir.unwrap_or(Path::new(""));
    let joined_path = parent_dir.join(path);
    if let Ok(abs_path) = std::fs::canonicalize(&joined_path) {
        return Ok(abs_path.to_string_lossy().to_string());
    }
    if let Ok(cwd) = env::current_dir() {
        return Ok(cwd.join(joined_path).clean().to_string_lossy().to_string());
    }

    Ok(joined_path.clean().to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::Config;

    #[test]
    fn test_from_json() {
        let content = std::fs::read_to_string(tailcall_fixtures::generator::SIMPLE_JSON).unwrap();
        let config: Config = serde_json::from_str(&content).unwrap();

        assert_debug_snapshot!(&config);
    }

    #[test]
    fn test_resolve_paths() {
        let file_path = tailcall_fixtures::generator::SIMPLE_JSON;
        let content = std::fs::read_to_string(tailcall_fixtures::generator::SIMPLE_JSON).unwrap();
        let config: Config = serde_json::from_str(&content).unwrap();
        let config = config.resolve_paths(file_path);
        assert!(config.is_ok());
    }
}
