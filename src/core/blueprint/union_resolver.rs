use crate::core::blueprint::FieldDefinition;
use crate::core::config;
use crate::core::config::{ConfigModule, Field};
use crate::core::ir::model::IR;
use crate::core::ir::Discriminator;
use crate::core::try_fold::TryFold;
use crate::core::valid::{Valid, Validator};

fn compile_union_resolver(
    union_name: &str,
    union_definition: &config::Union,
) -> Valid<Discriminator, String> {
    let typename_field = union_definition
        .discriminate
        .as_ref()
        .map(|d| d.field.clone());

    Discriminator::new(
        union_name.to_string(),
        union_definition.types.iter().cloned().collect(),
        typename_field,
    )
}

pub fn update_union_resolver<'a>(
) -> TryFold<'a, (&'a ConfigModule, &'a Field, &'a config::Type, &'a str), FieldDefinition, String>
{
    TryFold::<(&ConfigModule, &Field, &config::Type, &str), FieldDefinition, String>::new(
        |(config, field, _, _), mut b_field| {
            let Some(union_definition) = config.find_union(field.type_of.name()) else {
                return Valid::succeed(b_field);
            };

            compile_union_resolver(field.type_of.name(), union_definition).map(|discriminator| {
                b_field.resolver = Some(
                    b_field
                        .resolver
                        .unwrap_or(IR::ContextPath(vec![b_field.name.clone()])),
                );
                b_field.map_expr(move |expr| IR::Discriminate(discriminator, expr.into()));
                b_field
            })
        },
    )
}
