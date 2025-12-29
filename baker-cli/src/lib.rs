//! # Baker CLI
//!
//! Command-line interface for the Baker project scaffolding tool.

pub mod answers;
pub mod args;
pub mod processor;
pub mod runner;

pub use args::{get_args, get_log_level_from_verbose, Args, SkipConfirm};
pub use runner::run;
