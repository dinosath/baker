use crate::error::Result;
use crate::loader::interface::TemplateLoader;
use crate::loader::{git::GitLoader, local::LocalLoader};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod git;
pub mod interface;
pub mod local;

#[derive(Debug)]
pub enum TemplateSource {
    /// Local filesystem template path
    FileSystem(PathBuf),
    /// Git repository URL (HTTPS or SSH)
    Git(String),
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::FileSystem(path) => {
                write!(f, "local path: '{}'", path.display())
            }
            TemplateSource::Git(repo) => write!(f, "git repository: '{repo}'"),
        }
    }
}

/// Metadata about the template source captured at load time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TemplateSourceInfo {
    /// A local filesystem template with a content hash.
    Filesystem {
        /// Absolute path to the template directory.
        path: String,
        /// SHA-256 hex digest of all template file contents (excluding .bakerignore patterns).
        hash: String,
    },
    /// A git repository template.
    Git {
        /// The original URL used to clone the repository.
        url: String,
        /// Full commit SHA at HEAD when the template was cloned.
        commit: String,
        /// Tag pointing at HEAD, if any.
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
    },
}

/// The result of loading a template: the on-disk path plus source metadata.
#[derive(Debug)]
pub struct LoadedTemplate {
    /// Path to the template directory on disk.
    pub root: PathBuf,
    /// Metadata about where the template came from.
    pub source: TemplateSourceInfo,
}

/// Creates a TemplateFactory from a string path or URL and loads the template.
///
/// # Arguments
/// * `s` - String containing path or git URL
/// * `skip_overwrite_check` - Whether to skip confirmation for overwriting existing directories
///
/// # Returns
/// * `Result<LoadedTemplate>` - Loaded template with path and source metadata
pub fn get_template(s: &str, skip_overwrite_check: bool) -> Result<LoadedTemplate> {
    let source = if GitLoader::<&str>::is_git_url(s) {
        TemplateSource::Git(s.to_string())
    } else {
        TemplateSource::FileSystem(PathBuf::from(s))
    };

    match source {
        TemplateSource::Git(repo) => {
            GitLoader::new(repo.clone(), skip_overwrite_check).load()
        }
        TemplateSource::FileSystem(path) => LocalLoader::new(path.clone()).load(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_source_display() {
        let fs_source = TemplateSource::FileSystem(PathBuf::from("/path/to/template"));
        assert_eq!(format!("{fs_source}"), "local path: '/path/to/template'");

        let git_source = TemplateSource::Git("git@github.com:user/repo".to_string());
        assert_eq!(format!("{git_source}"), "git repository: 'git@github.com:user/repo'");
    }

    #[test]
    fn test_get_template_uses_local_loader_for_filesystem_paths() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("baker.yaml"),
            "schemaVersion: v1\nquestions: {}\n",
        )
        .unwrap();

        let loaded = get_template(tmp.path().to_str().unwrap(), true).unwrap();

        assert_eq!(loaded.root, tmp.path().to_path_buf());
        match loaded.source {
            TemplateSourceInfo::Filesystem { path, hash } => {
                assert_eq!(path, tmp.path().to_string_lossy().to_string());
                assert!(!hash.is_empty());
            }
            _ => panic!("expected filesystem template source"),
        }
    }
}
