mod command;
mod error;
mod fmt;
pub mod server;
mod tc;

pub use error::CLIError;
pub use tc::run;

use crate::config::Upstream;
use crate::io::{EnvIO, FileIO, HttpIO};

pub(crate) mod env;
pub(crate) mod file;
pub(crate) mod http;

// Provides access to env in native rust environment
pub fn init_env() -> impl EnvIO {
  env::EnvNative::init()
}

// Provides access to file system in native rust environment
pub fn init_file() -> impl FileIO {
  file::NativeFileIO::init()
}

// Provides access to http in native rust environment
pub fn init_http(upstream: &Upstream) -> impl HttpIO + Default + Clone {
  http::HttpNative::init(upstream)
}

// Provides access to http in native rust environment
pub fn init_http2_only(upstream: &Upstream) -> impl HttpIO + Default + Clone {
  http::HttpNative::init(&upstream.clone().http2_only(true))
}
