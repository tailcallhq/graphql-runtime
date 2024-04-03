use crate::blueprint::*;
use crate::config;
use crate::config::Field;
use crate::lambda::{Context, Expression};
use crate::try_fold::TryFold;
use crate::valid::Valid;

pub fn update_modify<'a>(
) -> TryFold<'a, (&'a ConfigModule, &'a Field, &'a config::Type, &'a str), FieldDefinition, String>
{
    TryFold::<(&ConfigModule, &Field, &config::Type, &'a str), FieldDefinition, String>::new(
        |(config, field, type_of, _), mut b_field| {
            if let Some(modify) = field.modify.as_ref() {
                if let Some(new_name) = &modify.name {
                    for name in type_of.implements.iter() {
                        let interface = config.types.get(name);
                        if let Some(interface) = interface {
                            if interface.fields.iter().any(|(name, _)| name == new_name) {
                                return Valid::fail(
                                    "Field is already implemented from interface".to_string(),
                                );
                            }
                        }
                    }
                    b_field.resolver = Some(b_field.resolver.unwrap_or(Expression::Context(
                        Context::Path(vec![b_field.name.clone()]),
                    )));
                    b_field = b_field.name(new_name.clone());
                }
            }
            Valid::succeed(b_field)
        },
    )
}
