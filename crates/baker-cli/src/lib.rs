//! # Baker CLI
//!
//! Command-line interface for the Baker project scaffolding tool.

pub mod answers;
pub mod args;
pub mod processor;
pub mod runner;
pub mod store;

#[cfg(feature = "mcp")]
pub mod mcp;

pub use args::{
    get_args, get_cli, get_log_level_from_verbose, Args, Cli, Commands, GenerateArgs,
    InfoArgs, InstallArgs, RemoveArgs, SkipConfirm,
};
pub use runner::run;
pub use store::TemplateStore;

#[cfg(feature = "mcp")]
pub use mcp::{run_mcp_server, BakerHandler};
