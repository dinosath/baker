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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_config_validation_error_display() {
        let err = Error::ConfigValidation("missing required field".to_string());
        assert_eq!(format!("{err}"), "Config validation failed: missing required field");
    }

    #[test]
    fn test_config_not_found_error_display() {
        let err = Error::ConfigNotFound {
            template_dir: "/path/to/template".to_string(),
            config_files: "baker.json, baker.yaml".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Configuration file not found in '/path/to/template'. Expected one of: baker.json, baker.yaml"
        );
    }

    #[test]
    fn test_json_parse_error_from() {
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let err = Error::from(json_err);
        assert!(matches!(err, Error::JSONParseError(_)));
        assert!(format!("{err}").contains("Failed to parse JSON"));
    }

    #[test]
    fn test_yaml_parse_error_from() {
        let yaml_err: serde_yaml::Error =
            serde_yaml::from_str::<serde_yaml::Value>(":\ninvalid").unwrap_err();
        let err = Error::from(yaml_err);
        assert!(matches!(err, Error::YAMLParseError(_)));
        assert!(format!("{err}").contains("Failed to parse YAML"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::IoError(_)));
        assert!(format!("{err}").contains("IO operation failed"));
    }

    #[test]
    fn test_glob_set_parse_error_from() {
        let glob_err = globset::GlobBuilder::new("[invalid").build().unwrap_err();
        let err = Error::from(glob_err);
        assert!(matches!(err, Error::GlobSetParseError(_)));
        assert!(format!("{err}").contains("Failed to parse glob pattern"));
    }

    #[test]
    fn test_output_directory_exists_error_display() {
        let err = Error::OutputDirectoryExistsError {
            output_dir: "/path/to/output".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Output directory '/path/to/output' already exists. Use --force to overwrite it."
        );
    }

    #[test]
    fn test_template_does_not_exist_error_display() {
        let err = Error::TemplateDoesNotExistsError {
            template_dir: "/path/to/template".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "Template directory '/path/to/template' does not exist"
        );
    }

    #[test]
    fn test_process_error_display() {
        let err = Error::ProcessError {
            source_path: "/path/to/file".to_string(),
            e: "invalid path".to_string(),
        };
        assert_eq!(format!("{err}"), "Cannot process path '/path/to/file': invalid path");
    }

    #[test]
    fn test_other_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("custom error message");
        let err = Error::from(anyhow_err);
        assert!(matches!(err, Error::Other(_)));
        assert_eq!(format!("{err}"), "custom error message");
    }

    #[test]
    fn test_minijinja_error_from() {
        let mut env = minijinja::Environment::new();
        // Add a template with a syntax error that will fail during evaluation
        env.add_template("test", "{% for x in %}{% endfor %}").unwrap_err();
        // Actually, let's just create an error directly by compiling invalid template
        let template_err = minijinja::Environment::new()
            .template_from_str("{% for x in %}{% endfor %}")
            .unwrap_err();
        let err = Error::from(template_err);
        assert!(matches!(err, Error::MinijinjaError(_)));
        assert!(format!("{err}").contains("Template rendering failed"));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = Error::ConfigValidation("test".to_string());
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("ConfigValidation"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }
        fn returns_err() -> Result<i32> {
            Err(Error::ConfigValidation("test".to_string()))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_result_with_custom_error_type() {
        fn returns_custom_err() -> Result<i32, String> {
            Err("custom error".to_string())
        }

        assert_eq!(returns_custom_err().unwrap_err(), "custom error");
    }

    #[test]
    fn test_walkdir_error_from() {
        // Create a WalkDir error by trying to walk a non-existent path with specific options
        let walkdir_result: std::result::Result<Vec<_>, _> =
            walkdir::WalkDir::new("/nonexistent/path/that/does/not/exist/at/all")
                .into_iter()
                .collect();

        // WalkDir doesn't error immediately on non-existent paths,
        // but we can still test the From impl
        if let Err(walkdir_err) = walkdir_result {
            let err = Error::from(walkdir_err);
            assert!(matches!(err, Error::WalkdirError(_)));
            assert!(format!("{err}").contains("File system traversal failed"));
        }
    }

    #[test]
    fn test_exit_codes_constant() {
        assert_eq!(exit_codes::SUCCESS, 0);
        assert_eq!(exit_codes::FAILURE, 1);
    }
}
