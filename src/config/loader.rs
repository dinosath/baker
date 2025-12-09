//! Configuration loading and management

use crate::config::question::Question;
use crate::constants::{
    CONFIG_FILENAMES, DEFAULT_LOOP_CONTENT_SEPARATOR, DEFAULT_LOOP_SEPARATOR,
    DEFAULT_POST_HOOK, DEFAULT_PRE_HOOK, DEFAULT_TEMPLATE_SUFFIX,
};
use crate::error::{Error, Result};
use crate::ext::PathExt;
use indexmap::IndexMap;
use serde::Deserialize;
use std::path::Path;

/// Main configuration structure holding all questions
#[derive(Debug, Deserialize)]
pub struct ConfigV1 {
    #[serde(default = "get_default_template_suffix")]
    pub template_suffix: String,
    #[serde(default = "get_default_loop_separator")]
    pub loop_separator: String,
    #[serde(default = "get_default_loop_content_separator")]
    pub loop_content_separator: String,
    #[serde(default = "get_default_template_globs")]
    pub template_globs: Vec<String>,
    #[serde(default)]
    pub import_root: Option<String>,
    #[serde(default)]
    pub questions: IndexMap<String, Question>,
    #[serde(default = "get_default_post_hook_filename")]
    pub post_hook_filename: String,
    #[serde(default = "get_default_pre_hook_filename")]
    pub pre_hook_filename: String,
    #[serde(default = "get_default_post_hook_runner")]
    pub post_hook_runner: Vec<String>,
    #[serde(default = "get_default_pre_hook_runner")]
    pub pre_hook_runner: Vec<String>,
    #[serde(default = "get_default_follow_symlinks")]
    pub follow_symlinks: bool,
}

impl ConfigV1 {
    pub fn validate(&self) -> Result<(), Error> {
        if self.template_suffix.is_empty() {
            return Err(Error::ConfigValidation(
                "template_suffix must not be empty".into(),
            ));
        }
        if !self.template_suffix.starts_with('.') || self.template_suffix.len() < 2 {
            return Err(Error::ConfigValidation("template_suffix must start with '.' and have at least 1 character after it".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "schemaVersion")]
pub enum Config {
    #[serde(rename = "v1")]
    V1(ConfigV1),
}

impl Config {
    pub fn load_config<P: AsRef<Path>>(template_root: P) -> Result<Self> {
        let template_root = template_root.as_ref().to_path_buf();
        let template_dir = template_root.to_str_checked()?.to_string();

        for config_file_name in CONFIG_FILENAMES.iter() {
            let config_file_path = template_root.join(config_file_name);

            if config_file_path.exists() {
                let content = std::fs::read_to_string(config_file_path)?;
                let config: Config = match *config_file_name {
                    "baker.json" => serde_json::from_str(&content)?,
                    "baker.yaml" | "baker.yml" => serde_yaml::from_str(&content)?,
                    _ => unreachable!(),
                };

                return Ok(config);
            }
        }

        Err(Error::ConfigNotFound {
            template_dir,
            config_files: CONFIG_FILENAMES.join(", "),
        })
    }
}

fn get_default_template_suffix() -> String {
    DEFAULT_TEMPLATE_SUFFIX.to_string()
}

fn get_default_template_globs() -> Vec<String> {
    Vec::new()
}

fn get_default_post_hook_runner() -> Vec<String> {
    Vec::new()
}

fn get_default_post_hook_filename() -> String {
    DEFAULT_POST_HOOK.to_string()
}

fn get_default_pre_hook_filename() -> String {
    DEFAULT_PRE_HOOK.to_string()
}

fn get_default_pre_hook_runner() -> Vec<String> {
    Vec::new()
}

fn get_default_loop_separator() -> String {
    DEFAULT_LOOP_SEPARATOR.to_string()
}

fn get_default_loop_content_separator() -> String {
    DEFAULT_LOOP_CONTENT_SEPARATOR.to_string()
}

fn get_default_follow_symlinks() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runners_default_to_empty_arrays() {
        let raw = r#"
schemaVersion: v1
pre_hook_filename: hooks/pre.sh
post_hook_filename: hooks/post.sh
questions: {}
"#;

        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;

        assert!(cfg.pre_hook_runner.is_empty());
        assert!(cfg.post_hook_runner.is_empty());
    }

    #[test]
    fn runners_parse_array_arguments() {
        let raw = r#"
schemaVersion: v1
pre_hook_filename: hooks/pre.ps1
pre_hook_runner: ["powershell", "-File"]
post_hook_filename: hooks/post.py
post_hook_runner: ["python3", "-u"]
questions: {}
"#;

        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;

        assert_eq!(
            cfg.pre_hook_runner,
            vec!["powershell".to_string(), "-File".to_string()]
        );
        assert_eq!(cfg.post_hook_runner, vec!["python3".to_string(), "-u".to_string()]);
    }

    #[test]
    fn follow_symlinks_defaults_false() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(!cfg.follow_symlinks);
    }

    #[test]
    fn follow_symlinks_parses_true() {
        let raw = r#"schemaVersion: v1
follow_symlinks: true
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(cfg.follow_symlinks);
    }

    #[test]
    fn import_root_defaults_to_none() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(cfg.import_root.is_none());
    }

    #[test]
    fn import_root_parses_path() {
        let raw = r#"schemaVersion: v1
import_root: "templates/shared"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.import_root, Some("templates/shared".to_string()));
    }

    #[test]
    fn import_root_parses_absolute_path() {
        let raw = r#"schemaVersion: v1
import_root: "/usr/local/templates"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.import_root, Some("/usr/local/templates".to_string()));
    }

    #[test]
    fn validate_accepts_valid_template_suffix() {
        let raw = r#"schemaVersion: v1
template_suffix: ".j2"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_template_suffix() {
        let raw = r#"schemaVersion: v1
template_suffix: ""
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("must not be empty"));
    }

    #[test]
    fn validate_rejects_template_suffix_without_dot() {
        let raw = r#"schemaVersion: v1
template_suffix: "j2"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        let result = cfg.validate();
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("must start with '.'"));
    }

    #[test]
    fn validate_rejects_template_suffix_with_only_dot() {
        let raw = r#"schemaVersion: v1
template_suffix: "."
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        let result = cfg.validate();
        assert!(result.is_err());
    }

    #[test]
    fn load_config_from_json_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("baker.json");
        std::fs::write(&config_path, r#"{"schemaVersion": "v1", "questions": {}}"#)
            .unwrap();

        let config = Config::load_config(temp_dir.path()).unwrap();
        let Config::V1(cfg) = config;
        assert_eq!(cfg.template_suffix, DEFAULT_TEMPLATE_SUFFIX);
    }

    #[test]
    fn load_config_from_yaml_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("baker.yaml");
        std::fs::write(&config_path, "schemaVersion: v1\nquestions: {}").unwrap();

        let config = Config::load_config(temp_dir.path()).unwrap();
        let Config::V1(cfg) = config;
        assert_eq!(cfg.template_suffix, DEFAULT_TEMPLATE_SUFFIX);
    }

    #[test]
    fn load_config_from_yml_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("baker.yml");
        std::fs::write(&config_path, "schemaVersion: v1\nquestions: {}").unwrap();

        let config = Config::load_config(temp_dir.path()).unwrap();
        let Config::V1(cfg) = config;
        assert_eq!(cfg.template_suffix, DEFAULT_TEMPLATE_SUFFIX);
    }

    #[test]
    fn load_config_returns_error_when_no_config_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = Config::load_config(temp_dir.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Configuration file not found"));
    }

    #[test]
    fn load_config_prefers_json_over_yaml() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        // Create both json and yaml files
        std::fs::write(
            temp_dir.path().join("baker.json"),
            r#"{"schemaVersion": "v1", "template_suffix": ".json.j2", "questions": {}}"#,
        )
        .unwrap();
        std::fs::write(
            temp_dir.path().join("baker.yaml"),
            "schemaVersion: v1\ntemplate_suffix: \".yaml.j2\"\nquestions: {}",
        )
        .unwrap();

        let config = Config::load_config(temp_dir.path()).unwrap();
        let Config::V1(cfg) = config;
        // JSON should be loaded first based on CONFIG_FILENAMES order
        assert_eq!(cfg.template_suffix, ".json.j2");
    }

    #[test]
    fn config_v1_debug_impl() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        let debug_str = format!("{:?}", cfg);
        assert!(debug_str.contains("ConfigV1"));
        assert!(debug_str.contains("template_suffix"));
    }

    #[test]
    fn config_enum_debug_impl() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("V1"));
    }

    #[test]
    fn default_template_globs_is_empty() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert!(cfg.template_globs.is_empty());
    }

    #[test]
    fn template_globs_parses_correctly() {
        let raw = r#"schemaVersion: v1
template_globs:
  - "**/*.j2"
  - "includes/*.html"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.template_globs.len(), 2);
        assert_eq!(cfg.template_globs[0], "**/*.j2");
        assert_eq!(cfg.template_globs[1], "includes/*.html");
    }

    #[test]
    fn default_loop_separator() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.loop_separator, DEFAULT_LOOP_SEPARATOR);
    }

    #[test]
    fn default_loop_content_separator() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.loop_content_separator, DEFAULT_LOOP_CONTENT_SEPARATOR);
    }

    #[test]
    fn custom_hook_filenames() {
        let raw = r#"schemaVersion: v1
pre_hook_filename: "setup.sh"
post_hook_filename: "cleanup.sh"
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.pre_hook_filename, "setup.sh");
        assert_eq!(cfg.post_hook_filename, "cleanup.sh");
    }

    #[test]
    fn default_hook_filenames() {
        let raw = r#"schemaVersion: v1
questions: {}"#;
        let config: Config = serde_yaml::from_str(raw).expect("valid config");
        let Config::V1(cfg) = config;
        assert_eq!(cfg.pre_hook_filename, DEFAULT_PRE_HOOK);
        assert_eq!(cfg.post_hook_filename, DEFAULT_POST_HOOK);
    }
}
