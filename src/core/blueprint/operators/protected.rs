use tailcall_valid::{Valid, Validator};

use crate::core::blueprint::{Auth, FieldDefinition, Provider};
use crate::core::config::{self, ConfigModule, Field};
use crate::core::ir::model::IR;
use crate::core::try_fold::TryFold;

pub fn update_protected<'a>(
    type_name: &'a str,
) -> TryFold<'a, (&'a ConfigModule, &'a Field, &'a config::Type, &'a str), FieldDefinition, String>
{
    TryFold::<(&ConfigModule, &Field, &config::Type, &'a str), FieldDefinition, String>::new(
        |(config, field, type_, _), mut b_field| {
            if field.protected.is_some() // check the field itself has marked as protected
                || type_.protected.is_some() // check the type that contains current field
                || config // check that output type of the field is protected
                    .find_type(field.type_of.name())
                    .and_then(|type_| type_.protected.as_ref())
                    .is_some()
            {
                if config.input_types().contains(type_name) {
                    return Valid::fail("Input types can not be protected".to_owned());
                }

                if !config.extensions().has_auth() {
                    return Valid::fail(
                        "@protected operator is used but there is no @link definitions for auth providers".to_owned(),
                    );
                }

                // Used to collect the providers that are used in the field
                Provider::from_config_module(config)
                    .and_then(|auth_providers| {
                        // FIXME: add trace information in the error
                        let mut field_protection = field
                            .protected
                            .clone()
                            .and_then(|protect| protect.id)
                            .unwrap_or_default();

                        let type_protection = type_
                            .protected
                            .clone()
                            .and_then(|protect| protect.id)
                            .unwrap_or_default();

                        field_protection.extend(type_protection);

                        let mut protection = field_protection;

                        if protection.is_empty() {
                            // If no protection is defined, use all providers
                            protection = auth_providers.keys().cloned().collect::<Vec<_>>();
                        }

                        Valid::from_iter(protection.iter(), |id| {
                            if let Some(provider) = auth_providers.get(id) {
                                Valid::succeed(Auth::Provider(provider.clone()))
                            } else {
                                Valid::fail(format!("Auth provider {} not found", id))
                            }
                        })
                    })
                    .map(|provider| {
                        let auth = provider.into_iter().reduce(|left, right| left.and(right));

                        if let (Some(auth), Some(resolver)) = (auth, b_field.resolver.clone()) {
                            b_field.resolver = Some(IR::Protect(auth, Box::new(resolver)));
                        }

                        b_field
                    })
            } else {
                Valid::succeed(b_field)
            }
        },
    )
}
