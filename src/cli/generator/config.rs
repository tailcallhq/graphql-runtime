use std::env;
use std::marker::PhantomData;
use std::path::Path;

use derive_setters::Setters;
use path_clean::PathClean;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::core::config::{self, ConfigReaderContext};
use crate::core::http::SerializableHeaderMap;
use crate::core::is_default;
use crate::core::mustache::Mustache;

#[derive(Deserialize, Serialize, Debug, Default, Setters)]
#[serde(rename_all = "camelCase")]
pub struct Config<Status = UnResolved> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<Input<Status>>,
    pub output: Output<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<Preset>,
    pub schema: Schema,
}

#[derive(Clone, Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    merge_type: Option<f32>,
    consolidate_url: Option<f32>,
}

impl From<Preset> for config::transformer::Preset {
    fn from(val: Preset) -> Self {
        let mut preset = config::transformer::Preset::default();
        if let Some(merge_type) = val.merge_type {
            preset = preset.merge_type(merge_type);
        }

        if let Some(consolidate_url) = val.consolidate_url {
            preset = preset.consolidate_url(consolidate_url);
        }

        preset
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(transparent)]
pub struct Location<A>(
    #[serde(skip_serializing_if = "Location::is_empty")] pub String,
    #[serde(skip)] PhantomData<A>,
);

impl<A> Location<A> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Location<UnResolved> {
    fn into_resolved(self, parent_dir: Option<&Path>) -> Location<Resolved> {
        let path = {
            let path = self.0.as_str();
            if Url::parse(path).is_ok() || Path::new(path).is_absolute() {
                path.to_string()
            } else {
                let parent_dir = parent_dir.unwrap_or(Path::new(""));
                let joined_path = parent_dir.join(path);
                if let Ok(abs_path) = std::fs::canonicalize(&joined_path) {
                    abs_path.to_string_lossy().to_string()
                } else if let Ok(cwd) = env::current_dir() {
                    cwd.join(joined_path).clean().to_string_lossy().to_string()
                } else {
                    joined_path.clean().to_string_lossy().to_string()
                }
            }
        };
        Location(path, PhantomData)
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Input<Status = UnResolved> {
    #[serde(flatten)]
    pub source: Source<Status>,
    pub field_name: String,
    pub operation: Operation,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Source<Status = UnResolved> {
    Curl {
        src: Location<Status>,
        #[serde(default, skip_serializing_if = "is_default")]
        headers: Option<SerializableHeaderMap>,
    },
    Proto {
        src: Location<Status>,
    },
    Config {
        src: Location<Status>,
    },
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Output<Status = UnResolved> {
    #[serde(skip_serializing_if = "Location::is_empty")]
    pub path: Location<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<config::Source>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    #[default]
    Query,
    Mutation,
}

#[derive(Debug)]
pub enum Resolved {}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct UnResolved {}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation: Option<String>,
}

impl Output<UnResolved> {
    pub fn resolve(self, parent_dir: Option<&Path>) -> anyhow::Result<Output<Resolved>> {
        Ok(Output {
            format: self.format,
            path: self.path.into_resolved(parent_dir),
        })
    }
}

impl Source<UnResolved> {
    pub fn resolve(
        self,
        parent_dir: Option<&Path>,
        config_reader: &ConfigReaderContext,
    ) -> anyhow::Result<Source<Resolved>> {
        match self {
            Source::Curl { src, mut headers } => {
                // Resolve the header values with mustache template.
                let resolved_headers = if let Some(original_headers) = &mut headers {
                    let mut cloned_headers = original_headers.headers().clone();
                    for header_value in cloned_headers.values_mut() {
                        *header_value = reqwest::header::HeaderValue::from_str(
                            &Mustache::parse(header_value.to_str()?)?.render(config_reader),
                        )?;
                    }
                    Some(SerializableHeaderMap::new(cloned_headers))
                } else {
                    None
                };

                let resolved_path = src.into_resolved(parent_dir);
                Ok(Source::Curl { src: resolved_path, headers: resolved_headers })
            }
            Source::Proto { src, .. } => {
                let resolved_path = src.into_resolved(parent_dir);
                Ok(Source::Proto { src: resolved_path })
            }
            Source::Config { src, .. } => {
                let resolved_path = src.into_resolved(parent_dir);
                Ok(Source::Config { src: resolved_path })
            }
        }
    }
}

impl Input<UnResolved> {
    pub fn resolve(
        self,
        parent_dir: Option<&Path>,
        config_reader: &ConfigReaderContext,
    ) -> anyhow::Result<Input<Resolved>> {
        let resolved_source = self.source.resolve(parent_dir, config_reader)?;
        Ok(Input {
            source: resolved_source,
            field_name: self.field_name,
            operation: self.operation,
        })
    }
}

impl Config {
    /// Resolves all the relative paths present inside the GeneratorConfig.
    pub fn into_resolved(
        self,
        config_path: &str,
        config_reader: ConfigReaderContext,
    ) -> anyhow::Result<Config<Resolved>> {
        let parent_dir = Some(Path::new(config_path).parent().unwrap_or(Path::new("")));

        let inputs = self
            .inputs
            .into_iter()
            .map(|input| input.resolve(parent_dir, &config_reader))
            .collect::<anyhow::Result<Vec<Input<Resolved>>>>()?;

        let output = self.output.resolve(parent_dir)?;

        Ok(Config { inputs, output, schema: self.schema, preset: self.preset })
    }
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    use super::*;

    fn location<S: AsRef<str>>(s: S) -> Location<UnResolved> {
        Location(s.as_ref().to_string(), PhantomData)
    }

    #[test]
    fn test_config_codec() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("user-agent"),
            HeaderValue::from_static("tailcall-v1"),
        );
        let headers = SerializableHeaderMap::new(headers);
        let config = Config::default().inputs(vec![Input {
            field_name: "test".to_string(),
            operation: Operation::Query,
            source: Source::Curl { src: location("https://example.com"), headers: Some(headers) },
        }]);
        let actual = serde_json::to_string_pretty(&config).unwrap();
        insta::assert_snapshot!(actual)
    }
}
