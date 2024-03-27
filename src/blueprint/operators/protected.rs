use crate::blueprint::FieldDefinition;
use crate::config::{self, ConfigModule, Field};
use crate::lambda::{Context, Expression};
use crate::try_fold::TryFold;
use crate::valid::Valid;

pub fn update_protected<'a>(
) -> TryFold<'a, (&'a ConfigModule, &'a Field, &'a config::Type, &'a str), FieldDefinition, String>
{
    TryFold::<(&ConfigModule, &Field, &config::Type, &'a str), FieldDefinition, String>::new(
        |(config, field, type_, _), mut b_field| {
            if field.protected.is_some() || type_.protected.is_some() {
                if !config.extensions.has_auth() {
                    return Valid::fail(
                        "@protected operator is used but there is no @link definitions for auth providers".to_owned(),
                    );
                }

                b_field.resolver = Some(Expression::Protect(Box::new(
                    b_field
                        .resolver
                        .unwrap_or(Expression::Context(Context::Value)),
                )));
            }

            Valid::succeed(b_field)
        },
    )
}
