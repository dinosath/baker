//! # Baker Core
//!
//! Core library for the Baker project scaffolding tool.
//!
//! This crate contains all the core components used for template processing,
//! configuration management, and project generation.
//!
//! ## Modules
//!
//! - [`config`] - Configuration handling for Baker templates
//! - [`template`] - Core template processing orchestration
//! - [`renderer`] - Template parsing and rendering functionality
//! - [`prompt`] - User input and interaction handling
//! - [`loader`] - Template source management (local/git)
//! - [`context`] - Generation context for template processing
//! - [`hooks`] - Hook script execution
//! - [`error`] - Custom error types
//! - [`constants`] - Application-wide constants
//! - [`ignore`] - .bakerignore file processing
//! - [`ext`] - Extension traits for built-in Rust types
//! - [`types`] - Common types

/// Application-wide constants.
pub mod constants;

/// Common types used across the crate.
pub mod types;

/// Defines custom error types.
pub mod error;

/// Generation context for template processing.
pub mod context;

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

/// Hook script execution.
pub mod hooks;

/// Extension traits for built-in Rust types.
pub mod ext;
