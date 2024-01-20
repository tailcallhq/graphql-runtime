mod definitions;
mod from_config;
mod operators;
mod schema;
mod server;
mod upstream;

pub use definitions::*;
pub use from_config::*;
pub use operators::*;
pub use schema::*;
pub use server::*;
pub use upstream::*;

use super::Type;
use crate::config::{Arg, Config, Field};
use crate::try_fold::TryFold;

pub type TryFoldConfig<'a, A> = TryFold<'a, Config, A, String>;

pub(crate) trait TypeLike {
  fn name(&self) -> &str;
  fn list(&self) -> bool;
  fn non_null(&self) -> bool;
  fn list_type_required(&self) -> bool;
}

impl TypeLike for Field {
  fn name(&self) -> &str {
    &self.type_of
  }

  fn list(&self) -> bool {
    self.list
  }

  fn non_null(&self) -> bool {
    self.required
  }

  fn list_type_required(&self) -> bool {
    self.list_type_required
  }
}

impl TypeLike for Arg {
  fn name(&self) -> &str {
    &self.type_of
  }

  fn list(&self) -> bool {
    self.list
  }

  fn non_null(&self) -> bool {
    self.required
  }

  fn list_type_required(&self) -> bool {
    false
  }
}

pub(crate) fn to_type<T>(field: &T, override_non_null: Option<bool>) -> Type
where
  T: TypeLike,
{
  let name = field.name();
  let list = field.list();
  let list_type_required = field.list_type_required();
  let non_null = if let Some(non_null) = override_non_null {
    non_null
  } else {
    field.non_null()
  };

  if list {
    Type::ListType {
      of_type: Box::new(Type::NamedType { name: name.to_string(), non_null: list_type_required }),
      non_null,
    }
  } else {
    Type::NamedType { name: name.to_string(), non_null }
  }
}

pub fn is_scalar(type_name: &str) -> bool {
  ["String", "Int", "Float", "Boolean", "ID", "JSON"].contains(&type_name)
}
