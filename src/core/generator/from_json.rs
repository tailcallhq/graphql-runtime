use derive_getters::Getters;
use serde_json::Value;
use url::Url;

use super::json::{self, GraphQLTypesGenerator};
use super::{NameGenerator, OperationType};
use crate::core::config::Config;
use crate::core::merge_right::MergeRight;
use crate::core::transform::{Transform, TransformerOps};
use crate::core::valid::{Valid, Validator};

#[derive(Getters)]
pub struct RequestSample {
    url: Url,
    response: Value,
    field_name: String,
    operation_type: OperationType,
}

impl RequestSample {
    pub fn new(url: Url, resp: Value, field_name: &str, operation_type: OperationType) -> Self {
        Self {
            url,
            response: resp,
            field_name: field_name.to_string(),
            operation_type,
        }
    }
}

pub struct FromJsonGenerator<'a> {
    request_samples: &'a [RequestSample],
    type_name_generator: &'a NameGenerator,
    query_name: &'a Option<String>,
    mutation_name: &'a Option<String>,
}

impl<'a> FromJsonGenerator<'a> {
    pub fn new(
        request_samples: &'a [RequestSample],
        type_name_generator: &'a NameGenerator,
        query_name: &'a Option<String>,
        mutation_name: &'a Option<String>,
    ) -> Self {
        Self {
            request_samples,
            type_name_generator,
            query_name,
            mutation_name,
        }
    }
}

impl Transform for FromJsonGenerator<'_> {
    type Value = Config;
    type Error = String;
    fn transform(&self, config: Self::Value) -> Valid<Self::Value, Self::Error> {
        let config_gen_req = self.request_samples;
        let type_name_gen = self.type_name_generator;

        Valid::from_iter(config_gen_req, |sample| {
            // these transformations are required in order to generate a base config.
            let operation_name = match &sample.operation_type {
                OperationType::Query => self.query_name.clone().unwrap_or("Query".to_owned()),
                OperationType::Mutation { .. } => {
                    self.mutation_name.clone().unwrap_or("Mutation".to_owned())
                }
            };

            GraphQLTypesGenerator::new(sample, type_name_gen, operation_name)
                .pipe(json::SchemaGenerator::new(
                    self.query_name,
                    self.mutation_name,
                ))
                .pipe(json::FieldBaseUrlGenerator::new(
                    &sample.url,
                    self.query_name,
                    self.mutation_name,
                ))
                .transform(config.clone())
        })
        .map(|configs| {
            configs
                .iter()
                .fold(config, |acc, c| acc.merge_right(c.clone()))
        })
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::core::config::transformer::Preset;
    use crate::core::generator::{FromJsonGenerator, NameGenerator, OperationType, RequestSample};
    use crate::core::transform::TransformerOps;
    use crate::core::valid::Validator;

    #[derive(Deserialize)]
    struct JsonFixture {
        url: String,
        response: serde_json::Value,
    }

    fn parse_json(path: &str) -> JsonFixture {
        let content = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    #[test]
    fn generate_config_from_json() -> anyhow::Result<()> {
        let mut request_samples = vec![];
        let fixtures = [
            "src/core/generator/tests/fixtures/json/incompatible_properties.json",
            "src/core/generator/tests/fixtures/json/list_incompatible_object.json",
            "src/core/generator/tests/fixtures/json/nested_list.json",
            "src/core/generator/tests/fixtures/json/nested_same_properties.json",
            "src/core/generator/tests/fixtures/json/incompatible_root_object.json",
        ];
        let field_name_generator = NameGenerator::new("f");
        for fixture in fixtures {
            let parsed_content = parse_json(fixture);
            request_samples.push(RequestSample::new(
                parsed_content.url.parse()?,
                parsed_content.response,
                &field_name_generator.next(),
                OperationType::Query,
            ));
        }

        let config = FromJsonGenerator::new(
            &request_samples,
            &NameGenerator::new("T"),
            &Some("Query".into()),
            &None,
        )
        .pipe(Preset::default())
        .generate()
        .to_result()?;

        insta::assert_snapshot!(config.to_sdl());
        Ok(())
    }
}
