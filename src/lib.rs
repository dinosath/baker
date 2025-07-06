/// Handles argument parsing.
pub mod cli;

/// Defines custom error types.
pub mod error;

/// Pre and post generation hook processing.
pub mod hooks;

/// Processes .bakerignore files to exclude specific paths.
pub mod ignore;

/// Template parsing and rendering functionality.
pub mod renderer;

/// User input and interaction handling.
pub mod prompt;

/// An abstraction that allows implementing a source for Baker templates.
pub mod loader;

/// Core template processing orchestration.
pub mod template;

/// Configuration handling for Baker templates.
pub mod config;

/// Extension traits for built-in Rust types.
pub mod ext;
