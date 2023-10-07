mod config;
mod from_document;
mod into_document;
mod key_values;
mod n_plus_one;
mod server;
mod source;

pub use config::*;
pub use key_values::*;
pub use server::*;
pub use source::*;

fn is_default<T: Default + Eq>(val: &T) -> bool {
  *val == T::default()
}
