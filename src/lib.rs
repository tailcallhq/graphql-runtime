mod core {
    // FIXME: use tailcall_core internally
    pub use tailcall_core::core::*;
}

#[cfg(feature = "cli")]
pub mod cli;
