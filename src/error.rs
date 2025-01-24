use std::process::ExitStatus;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON parse error: {0}.")]
    JSONParseError(#[from] serde_json::Error),

    #[error("YAML parse error: {0}.")]
    YAMLParseError(#[from] serde_yaml::Error),
    #[error("IO error: {0}.")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse .bakerignore file. Original error: {0}")]
    GlobSetParseError(#[from] globset::Error),

    #[error("Failed to clone repository. Original error: {0}")]
    Git2Error(#[from] git2::Error),

    #[error("Failed to render. Original error: {0}")]
    MinijinjaError(#[from] minijinja::Error),

    #[error("Failed to extract dir entry. Original error: {0}")]
    WalkdirError(#[from] walkdir::Error),

    #[error(
        "Configuration file not found. Searched in '{template_dir}' for: {config_files}"
    )]
    ConfigNotFound { template_dir: String, config_files: String },

    #[error("Hook script '{script}' failed with exit code: {status}")]
    HookExecutionError { script: String, status: ExitStatus },

    #[error("Cannot proceed: output directory '{output_dir}' already exists. Use --force to overwrite it.")]
    OutputDirectoryExistsError { output_dir: String },
    #[error("Cannot proceed: template directory '{template_dir}' does not exist.")]
    TemplateDoesNotExistsError { template_dir: String },

    #[error("Cannot process the source path: '{source_path}'. Original error: {e}")]
    ProcessError { source_path: String, e: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub fn default_error_handler(err: Error) {
    eprintln!("{}", err);
    std::process::exit(1);
}
