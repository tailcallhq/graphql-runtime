mod keyed_discriminator;
mod type_field_discriminator;

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use async_graphql::Value;
use keyed_discriminator::KeyedDiscriminator;
use type_field_discriminator::TypeFieldDiscriminator;

use crate::core::json::{JsonLike, JsonObjectLike};
use crate::core::valid::{Valid, Validator};

/// Resolver for type member of a union or interface.
#[derive(Debug, Clone)]
pub enum Discriminator {
    Keyed(KeyedDiscriminator),
    TypeField(TypeFieldDiscriminator),
}

impl Discriminator {
    pub fn new(
        type_name: String,
        types: BTreeSet<String>,
        typename_field: Option<String>,
    ) -> Valid<Self, String> {
        if let Some(typename_field) = typename_field {
            TypeFieldDiscriminator::new(type_name, types, typename_field).map(Self::TypeField)
        } else {
            KeyedDiscriminator::new(type_name, types).map(Self::Keyed)
        }
    }

    pub fn resolve_type(&self, value: Value) -> Result<Value> {
        // if typename is already present we return it
        if value.get_type_name().is_some() {
            return Ok(value);
        }

        match self {
            Discriminator::Keyed(keyed_discriminator) => {
                keyed_discriminator.resolve_and_set_type(value)
            }
            Discriminator::TypeField(type_field_discriminator) => {
                type_field_discriminator.resolve_and_set_type(value)
            }
        }
    }
}

pub trait TypedValue<'a> {
    type Error;

    fn get_type_name(&'a self) -> Option<&'a str>;
    fn set_type_name(&'a mut self, type_name: String) -> Result<(), Self::Error>;
}

const TYPENAME_FIELD: &str = "__typename";

impl<'json, T> TypedValue<'json> for T
where
    T: JsonLike<'json>,
{
    type Error = anyhow::Error;

    fn get_type_name(&'json self) -> Option<&'json str> {
        self.as_object()
            .and_then(|obj| obj.get_key(TYPENAME_FIELD))
            .and_then(|val| val.as_str())
    }

    fn set_type_name(&'json mut self, type_name: String) -> Result<(), Self::Error> {
        if let Some(obj) = self.as_object_mut() {
            obj.insert_key(TYPENAME_FIELD, T::string(type_name.into()));

            Ok(())
        } else {
            bail!("Expected object")
        }
    }
}
