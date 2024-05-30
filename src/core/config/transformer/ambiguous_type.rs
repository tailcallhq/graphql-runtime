use std::collections::HashMap;

use super::Transform;
use crate::core::config::ConfigModule;
use crate::core::valid::Valid;

/// Resolves the ambiguous types by renaming the input and
/// output types. The resolver function is called whenever is a conflict is
/// detected with the name that has a conflict. The returned value should return
/// a Resolution object containing the new input and output types.
/// The function will return a new ConfigModule with the resolved types.

pub struct Resolution {
    pub input: String,
    pub output: String,
}

pub struct AmbiguousType {
    resolver: Box<dyn Fn(&str) -> Resolution>,
}

impl Default for AmbiguousType {
    fn default() -> Self {
        Self::new(|v: &str| Resolution { input: format!("In_{}", v), output: v.to_string() })
    }
}

impl AmbiguousType {
    pub fn new(resolver: impl Fn(&str) -> Resolution + 'static) -> Self {
        Self { resolver: Box::new(resolver) }
    }
}

fn insert_resolution(
    mut map: HashMap<String, Resolution>,
    current_name: &str,
    resolution: Resolution,
) -> HashMap<String, Resolution> {
    if resolution.input.eq(&resolution.output) {
        tracing::warn!(
            "Unable to resolve input and output type: {}",
            resolution.input
        );
    }

    if !map.contains_key(current_name) {
        map.entry(current_name.to_string()).or_insert(resolution);
    }

    map
}

impl Transform for AmbiguousType {
    fn transform(&self, mut this: ConfigModule) -> Valid<ConfigModule, String> {
        let mut resolution_map = HashMap::new();

        // iterate over intersection of input and output types
        for current_name in this.input_types.intersection(&this.output_types) {
            let resolution = (self.resolver)(current_name);

            resolution_map = insert_resolution(resolution_map, current_name, resolution);

            if let Some(ty) = this.config.types.get(current_name) {
                for field in ty.fields.values() {
                    for args in field.args.values() {
                        // if arg is of output type then it should be changed to that of newly
                        // created input type.
                        if this.output_types.contains(&args.type_of)
                            && !resolution_map.contains_key(&args.type_of)
                        {
                            let resolution = (self.resolver)(args.type_of.as_str());
                            resolution_map = insert_resolution(
                                resolution_map,
                                args.type_of.as_str(),
                                resolution,
                            );
                        }
                    }
                }
            }
        }

        // insert newly created types to the config.
        for (current_name, resolution) in &resolution_map {
            let input_name = &resolution.input;
            let output_name = &resolution.output;

            let og_ty = this.config.types.get(current_name).cloned();

            // remove old types
            this.config.types.remove(current_name);
            this.input_types.remove(current_name);
            this.output_types.remove(current_name);

            // add new types
            if let Some(og_ty) = og_ty {
                this.config.types.insert(input_name.clone(), og_ty.clone());
                this.input_types.insert(input_name.clone());

                this.config.types.insert(output_name.clone(), og_ty);
                this.output_types.insert(output_name.clone());
            }
        }

        let keys = this.config.types.keys().cloned().collect::<Vec<String>>();

        for k in keys {
            if let Some(ty) = this.config.types.get_mut(&k) {
                for field in ty.fields.values_mut() {
                    if let Some(resolution) = resolution_map.get(&field.type_of) {
                        if this.output_types.contains(&k) {
                            field.type_of.clone_from(&resolution.output);
                        } else if this.input_types.contains(&k) {
                            field.type_of.clone_from(&resolution.input);
                        }
                    }
                    for arg in field.args.values_mut() {
                        if let Some(resolution) = resolution_map.get(&arg.type_of) {
                            arg.type_of.clone_from(&resolution.input);
                        }
                    }
                }
            }
        }

        // FIXME: Validation failure should happen if the resolution is not possible
        Valid::succeed(this)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use maplit::hashset;

    use crate::core::config::transformer::{AmbiguousType, Resolution, Transform};
    use crate::core::config::{Config, ConfigModule, Type};
    use crate::core::generator::Source;
    use crate::core::valid::Validator;

    fn build_qry(mut config: Config) -> Config {
        let mut query = Type::default();
        let mut field1 =
            crate::core::config::Field { type_of: "Type1".to_string(), ..Default::default() };

        let arg1 = crate::core::config::Arg { type_of: "Type1".to_string(), ..Default::default() };

        field1.args.insert("arg1".to_string(), arg1);

        let arg2 = crate::core::config::Arg { type_of: "Type2".to_string(), ..Default::default() };

        field1.args.insert("arg2".to_string(), arg2);

        let mut field2 = field1.clone();
        field2.type_of = "Type2".to_string();

        query.fields.insert("field1".to_string(), field1);
        query.fields.insert("field2".to_string(), field2);

        config.types.insert("Query".to_string(), query);
        config = config.query("Query");

        config
    }

    #[test]
    fn test_resolve_ambiguous_types() {
        // Create a ConfigModule instance with ambiguous types
        let mut config = Config::default();

        let mut type1 = Type::default();
        let mut type2 = Type::default();
        let mut type3 = Type::default();

        type1.fields.insert(
            "name".to_string(),
            crate::core::config::Field::default().type_of("String".to_string()),
        );

        type2.fields.insert(
            "ty1".to_string(),
            crate::core::config::Field::default().type_of("Type1".to_string()),
        );

        type3.fields.insert(
            "ty1".to_string(),
            crate::core::config::Field::default().type_of("Type1".to_string()),
        );
        type3.fields.insert(
            "ty2".to_string(),
            crate::core::config::Field::default().type_of("Type2".to_string()),
        );

        config.types.insert("Type1".to_string(), type1);
        config.types.insert("Type2".to_string(), type2);
        config.types.insert("Type3".to_string(), type3);

        config = build_qry(config);

        let mut config_module = ConfigModule::from(config);

        config_module = AmbiguousType::new(|type_name: &str| -> Resolution {
            Resolution {
                input: format!("{}Input", type_name),
                output: format!("{}Output", type_name),
            }
        })
        .transform(config_module)
        .to_result()
        .unwrap();

        let actual = config_module
            .config
            .types
            .keys()
            .map(|s| s.as_str())
            .collect::<HashSet<_>>();

        let expected = maplit::hashset![
            "Query",
            "Type1Input",
            "Type1",
            "Type2Input",
            "Type2",
            "Type3"
        ];

        assert_eq!(actual, expected);
    }
    #[tokio::test]
    async fn test_resolve_ambiguous_news_types() -> anyhow::Result<()> {
        let gen = crate::core::generator::Generator::init(crate::core::runtime::test::init(None));
        let news = tailcall_fixtures::protobuf::NEWS;
        let config_module = gen.read_all(Source::Proto, &[news], "Query").await?;
        let actual = config_module
            .config
            .types
            .keys()
            .map(|s| s.as_str())
            .collect::<HashSet<_>>();

        let expected = hashset![
            "Query",
            "news__News",
            "news__NewsList",
            "news__NewsInput",
            "news__NewsId",
            "news__MultipleNewsId"
        ];
        assert_eq!(actual, expected);
        Ok(())
    }
}
