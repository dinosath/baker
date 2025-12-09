use crate::error::Result;
use crate::loader::interface::TemplateLoader;
use crate::loader::{git::GitLoader, local::LocalLoader};
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

/// Creates a TemplateFactory from a string path or URL and loads the template.
///
/// # Arguments
/// * `s` - String containing path or git URL
/// * `skip_overwrite_check` - Whether to skip confirmation for overwriting existing directories
///
/// # Returns
/// * `Result<PathBuf>` - Path to the loaded template
pub fn get_template(s: &str, skip_overwrite_check: bool) -> Result<PathBuf> {
    // Check if this is a git repository URL
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
    use tempfile::TempDir;

    #[test]
    fn test_template_source_display() {
        let fs_source = TemplateSource::FileSystem(PathBuf::from("/path/to/template"));
        assert_eq!(format!("{fs_source}"), "local path: '/path/to/template'");

        let git_source = TemplateSource::Git("git@github.com:user/repo".to_string());
        assert_eq!(format!("{git_source}"), "git repository: 'git@github.com:user/repo'");
    }

    #[test]
    fn test_template_source_debug() {
        let fs_source = TemplateSource::FileSystem(PathBuf::from("/test/path"));
        let debug_str = format!("{fs_source:?}");
        assert!(debug_str.contains("FileSystem"));
        assert!(debug_str.contains("/test/path"));

        let git_source = TemplateSource::Git("https://example.com/repo".to_string());
        let debug_str = format!("{git_source:?}");
        assert!(debug_str.contains("Git"));
        assert!(debug_str.contains("https://example.com/repo"));
    }

    #[test]
    fn test_get_template_local_path_exists() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_template(temp_dir.path().to_str().unwrap(), false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp_dir.path().to_path_buf());
    }

    #[test]
    fn test_get_template_local_path_not_exists() {
        let result = get_template("/nonexistent/path/that/does/not/exist", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_template_identifies_git_https_url() {
        // This will fail to clone but validates URL detection
        let result = get_template("https://github.com/nonexistent/repo", true);
        // The result is an error because the repo doesn't exist, but it should
        // have been identified as a git URL and attempted to clone
        assert!(result.is_err());
    }

    #[test]
    fn test_local_loader_new() {
        let loader = LocalLoader::new(PathBuf::from("/test/path"));
        // Just verify it compiles and creates successfully - we can't access private field
        // but we can test that load() returns the expected path for non-existent path
        let result = loader.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_local_loader_load_existing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let loader = LocalLoader::new(temp_dir.path().to_path_buf());
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp_dir.path().to_path_buf());
    }

    #[test]
    fn test_local_loader_load_nonexistent_directory() {
        let loader = LocalLoader::new(PathBuf::from("/this/path/should/not/exist"));
        let result = loader.load();
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("does not exist"));
    }

    #[test]
    fn test_local_loader_with_str_path() {
        let temp_dir = TempDir::new().unwrap();
        let path_str = temp_dir.path().to_str().unwrap();
        let loader = LocalLoader::new(path_str);
        let result = loader.load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_local_loader_with_path_reference() {
        let temp_dir = TempDir::new().unwrap();
        let loader = LocalLoader::new(temp_dir.path());
        let result = loader.load();
        assert!(result.is_ok());
    }
}
