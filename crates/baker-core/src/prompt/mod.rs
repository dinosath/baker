//! Interactive dialog utilities for user input.
//!
//! The prompt subsystem is split into layers:
//! - [`interface`]: transport-agnostic traits and configs.
//! - [`dialoguer`]: the default terminal implementation.
//! - [`handler`]: orchestration that chooses which prompt to display.
//! - [`context`]: immutable data passed to prompt providers.
//! - [`provider`]: convenience helpers exposed to the rest of the crate.

pub mod context;
pub mod dialoguer;
pub mod handler;
pub mod interface;
pub mod parser;
pub mod provider;

pub use context::PromptContext;
pub use interface::*;
pub use provider::{ask_question, confirm, get_prompt_provider, Prompter};
