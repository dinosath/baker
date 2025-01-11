use std::process::ExitStatus;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}.")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config file.")]
    ConfigParseError,

    #[error("Failed to parse .bakerignore file. Original error: {0}")]
    GlobSetParseError(#[from] globset::Error),

    #[error("Failed to clone repository. Original error: {0}")]
    Git2Error(#[from] git2::Error),

    #[error("Failed to render. Original error: {0}")]
    MinijinjaError(#[from] minijinja::Error),

    #[error("Template error: {0}.")]
    TemplateError(String),

    #[error("No configuration file found in '{template_dir}'. Tried: {config_files}.")]
    ConfigError { template_dir: String, config_files: String },

    /// When the Hook has executed but finished with an error.
    #[error("Hook execution failed with status: {status}")]
    HookExecutionError { status: ExitStatus },

    /// Represents validation failures in user input or data
    #[error("Validation error: {0}.")]
    ValidationError(String),

    /// Represents errors in processing .bakerignore files
    #[error("BakerIgnore error: {0}.")]
    BakerIgnoreError(String),

    #[error("Cannot proceed: output directory '{output_dir}' already exists. Use --force to overwrite it.")]
    OutputDirectoryExistsError { output_dir: String },
    #[error("Cannot proceed: template directory '{template_dir}' does not exist.")]
    TemplateDoesNotExistsError { template_dir: String },
    #[error("Cannot proceed: invalid type of template source.")]
    TemplateSourceInvalidError,

    #[error("Cannot process the source path: '{source_path}'. Original error: {e}")]
    ProcessError { source_path: String, e: String },
}

/// Convenience type alias for Results with BakerError as the error type.
///
/// # Type Parameters
/// * `T` - The type of the success value
pub type Result<T> = std::result::Result<T, Error>;

/// Default error handler that prints the error and exits the program.
///
/// # Arguments
/// * `err` - The BakerError to handle
///
/// # Behavior
/// Prints the error message to stderr and exits with status code 1
pub fn default_error_handler(err: Error) {
    eprintln!("{}", err);
    std::process::exit(1);
}
