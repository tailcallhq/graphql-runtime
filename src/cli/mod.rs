pub mod command;
mod error;
mod fmt;
pub mod generator;
#[cfg(feature = "js")]
pub mod javascript;
mod llm;
pub mod metrics;
pub mod runtime;
pub mod server;
mod tc;
pub mod telemetry;
pub(crate) mod update_checker;
pub use error::CLIError;
pub use tc::run::run;
