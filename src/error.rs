use dialoguer::Error as DialoguerError;
use std::process::ExitStatus;
use thiserror::Error;

use crate::constants::exit_codes;

/// Represents all possible errors that can occur in Baker
#[derive(Error, Debug)]
pub enum Error {
    // Configuration errors
    #[error("Config validation failed: {0}")]
    ConfigValidation(String),

    #[error(
        "Configuration file not found in '{template_dir}'. Expected one of: {config_files}"
    )]
    ConfigNotFound { template_dir: String, config_files: String },

    // User interaction errors
    #[error("Dialoguer error: {0}")]
    DialoguerError(#[from] DialoguerError),

    // Parsing errors
    #[error("Failed to parse JSON: {0}")]
    JSONParseError(#[from] serde_json::Error),

    #[error("Failed to parse YAML: {0}")]
    YAMLParseError(#[from] serde_yaml::Error),

    #[error("Failed to parse glob pattern in .bakerignore file: {0}")]
    GlobSetParseError(#[from] globset::Error),

    // System errors
    #[error("IO operation failed: {0}")]
    IoError(#[from] std::io::Error),

    #[error("File system traversal failed: {0}")]
    WalkdirError(#[from] walkdir::Error),

    // External tool errors
    #[error("Git operation failed: {0}")]
    Git2Error(#[from] git2::Error),

    #[error("Template rendering failed: {0}")]
    MinijinjaError(#[from] minijinja::Error),

    // Process execution errors
    #[error("Hook script '{script}' failed with exit code: {status}")]
    HookExecutionError { script: String, status: ExitStatus },

    // Business logic errors
    #[error(
        "Output directory '{output_dir}' already exists. Use --force to overwrite it."
    )]
    OutputDirectoryExistsError { output_dir: String },

    #[error("Template directory '{template_dir}' does not exist")]
    TemplateDoesNotExistsError { template_dir: String },

    #[error("Cannot process path '{source_path}': {e}")]
    ProcessError { source_path: String, e: String },

    // Generic errors
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Standard Result type for Baker operations
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Default error handler that prints the error message and exits with code 1
pub fn default_error_handler(err: Error) {
    log::error!("{err}");
    std::process::exit(exit_codes::FAILURE);
}
