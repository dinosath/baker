use dialoguer::Error as DialoguerError;
use std::process::ExitStatus;
use thiserror::Error;

/// Represents all possible errors that can occur in Baker
#[derive(Error, Debug)]
pub enum Error {
    #[error("Dialoguer error: {0}")]
    DialoguerError(#[from] DialoguerError),

    #[error("Failed to parse JSON: {0}")]
    JSONParseError(#[from] serde_json::Error),

    #[error("Failed to parse YAML: {0}")]
    YAMLParseError(#[from] serde_yaml::Error),

    #[error("IO operation failed: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse glob pattern in .bakerignore file: {0}")]
    GlobSetParseError(#[from] globset::Error),

    #[error("Git operation failed: {0}")]
    Git2Error(#[from] git2::Error),

    #[error("Template rendering failed: {0}")]
    MinijinjaError(#[from] minijinja::Error),

    #[error("File system traversal failed: {0}")]
    WalkdirError(#[from] walkdir::Error),

    #[error(
        "Configuration file not found in '{template_dir}'. Expected one of: {config_files}"
    )]
    ConfigNotFound { template_dir: String, config_files: String },

    #[error("Hook script '{script}' failed with exit code: {status}")]
    HookExecutionError { script: String, status: ExitStatus },

    #[error(
        "Output directory '{output_dir}' already exists. Use --force to overwrite it."
    )]
    OutputDirectoryExistsError { output_dir: String },

    #[error("Template directory '{template_dir}' does not exist")]
    TemplateDoesNotExistsError { template_dir: String },

    #[error("Cannot process path '{source_path}': {e}")]
    ProcessError { source_path: String, e: String },

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Standard Result type for Baker operations
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Default error handler that prints the error message and exits with code 1
pub fn default_error_handler(err: Error) {
    eprintln!("Error: {err}");
    std::process::exit(1);
}
