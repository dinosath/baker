//! Generated metadata file — written to the output directory after every generate run.

use crate::{
    config::ConfigV1, constants::DEFAULT_GENERATED_FILE_NAME, error::Result,
    loader::TemplateSourceInfo,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The top-level structure serialised to `.baker-generated.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakerGenerated {
    /// Schema version — always `"1"` for now.
    pub version: String,
    /// RFC3339 timestamp of when the generation ran.
    pub generated_at: String,
    /// Information about the template source (local path + hash, or git URL + commit/tag).
    pub template: TemplateSourceInfo,
    /// The answers collected during generation, serialised as a JSON value.
    pub answers: serde_json::Value,
}

impl BakerGenerated {
    /// Create a new record from the given source info and answers, timestamped now.
    pub fn new(template: TemplateSourceInfo, answers: serde_json::Value) -> Self {
        Self {
            version: "1".to_string(),
            generated_at: Utc::now().to_rfc3339(),
            template,
            answers,
        }
    }
}

/// Write a `BakerGenerated` record to `<output_dir>/<file_name>`.
pub fn write(output_dir: &Path, file_name: &str, data: &BakerGenerated) -> Result<()> {
    let path = output_dir.join(file_name);
    let yaml = serde_yaml::to_string(data)?;
    std::fs::write(&path, yaml)?;
    log::debug!("Wrote generated metadata to '{}'", path.display());
    Ok(())
}

/// Read a `BakerGenerated` record from `<dir>/<file_name>`.
///
/// Returns `Err(GeneratedFileNotFound)` when the file is absent.
pub fn read(dir: &Path, file_name: &str) -> Result<BakerGenerated> {
    let path = dir.join(file_name);
    if !path.exists() {
        return Err(crate::error::Error::GeneratedFileNotFound { path });
    }
    let content = std::fs::read_to_string(&path)?;
    let data: BakerGenerated = serde_yaml::from_str(&content)?;
    if data.version != "1" {
        return Err(crate::error::Error::UnsupportedGeneratedVersion {
            found: data.version,
        });
    }
    Ok(data)
}

/// Resolve the effective generated-file name from (in priority order):
/// 1. CLI flag (`cli_override`)
/// 2. Config field (`config_value`)
/// 3. Build-time default
pub fn resolve_file_name<'a>(
    cli_override: Option<&'a str>,
    config_value: Option<&'a str>,
) -> &'a str {
    cli_override.or(config_value).unwrap_or(DEFAULT_GENERATED_FILE_NAME)
}

/// Returns a copy of `answers` with secret (password) question keys removed.
///
/// Questions that have `secret: Some(...)` are considered password fields and
/// should not be persisted in plaintext. On `update`, these will be re-prompted.
pub fn strip_secret_answers(
    answers: &serde_json::Value,
    config: &ConfigV1,
) -> serde_json::Value {
    let mut filtered = answers.clone();
    if let Some(obj) = filtered.as_object_mut() {
        for (key, question) in &config.questions {
            if question.secret.is_some() {
                obj.remove(key);
            }
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::TemplateSourceInfo;
    use tempfile::TempDir;

    fn make_filesystem_source() -> TemplateSourceInfo {
        TemplateSourceInfo::Filesystem {
            path: "/tmp/my-template".to_string(),
            hash: "abc123".to_string(),
        }
    }

    #[test]
    fn round_trip_filesystem() {
        let tmp = TempDir::new().unwrap();
        let data = BakerGenerated::new(
            make_filesystem_source(),
            serde_json::json!({"name": "test", "debug": true}),
        );
        write(tmp.path(), ".baker-generated.yaml", &data).unwrap();
        let loaded = read(tmp.path(), ".baker-generated.yaml").unwrap();
        assert_eq!(loaded.version, "1");
        assert_eq!(loaded.answers["name"], "test");
    }

    #[test]
    fn round_trip_git() {
        let tmp = TempDir::new().unwrap();
        let data = BakerGenerated::new(
            TemplateSourceInfo::Git {
                url: "https://github.com/example/tpl".to_string(),
                commit: "deadbeef".to_string(),
                tag: Some("v1.0.0".to_string()),
            },
            serde_json::json!({}),
        );
        write(tmp.path(), ".baker-generated.yaml", &data).unwrap();
        let loaded = read(tmp.path(), ".baker-generated.yaml").unwrap();
        if let TemplateSourceInfo::Git { url, commit, tag } = loaded.template {
            assert_eq!(url, "https://github.com/example/tpl");
            assert_eq!(commit, "deadbeef");
            assert_eq!(tag, Some("v1.0.0".to_string()));
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn read_missing_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let result = read(tmp.path(), ".baker-generated.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn resolve_file_name_priority() {
        assert_eq!(resolve_file_name(Some("cli.yaml"), Some("config.yaml")), "cli.yaml");
        assert_eq!(resolve_file_name(None, Some("config.yaml")), "config.yaml");
        assert_eq!(resolve_file_name(None, None), DEFAULT_GENERATED_FILE_NAME);
    }

    #[test]
    fn read_rejects_unsupported_version() {
        let tmp = TempDir::new().unwrap();
        let yaml = "version: \"2\"\ngenerated_at: \"2024-01-01T00:00:00Z\"\ntemplate:\n  type: filesystem\n  path: /tmp/t\n  hash: abc\nanswers: {}\n";
        std::fs::write(tmp.path().join("meta.yaml"), yaml).unwrap();
        let err = read(tmp.path(), "meta.yaml").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("Unsupported") && msg.contains("2"),
            "expected version mismatch error; got: {msg}"
        );
    }

    #[test]
    fn strip_secret_answers_removes_password_keys() {
        use crate::config::Config;

        let raw = r#"
schemaVersion: v1
questions:
  name:
    type: str
    help: Your name
    default: World
  password:
    type: str
    help: Enter password
    secret:
      confirm: true
  api_key:
    type: str
    help: API key
    secret:
      confirm: false
"#;
        let config: Config = serde_yaml::from_str(raw).unwrap();
        let Config::V1(config) = config;

        let answers = serde_json::json!({
            "name": "Alice",
            "password": "hunter2",
            "api_key": "secret123"
        });

        let stripped = strip_secret_answers(&answers, &config);
        assert_eq!(stripped.get("name").unwrap(), "Alice");
        assert!(stripped.get("password").is_none(), "password should be stripped");
        assert!(stripped.get("api_key").is_none(), "api_key should be stripped");
    }

    #[test]
    fn strip_secret_answers_noop_when_no_secrets() {
        use crate::config::Config;

        let raw = r#"
schemaVersion: v1
questions:
  name:
    type: str
    help: Your name
    default: World
"#;
        let config: Config = serde_yaml::from_str(raw).unwrap();
        let Config::V1(config) = config;

        let answers = serde_json::json!({"name": "Bob"});
        let stripped = strip_secret_answers(&answers, &config);
        assert_eq!(stripped, answers);
    }
}
