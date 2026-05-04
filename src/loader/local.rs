use crate::error::{Error, Result};
use crate::ignore::parse_bakerignore_file;
use crate::loader::interface::TemplateLoader;
use crate::loader::{LoadedTemplate, TemplateSourceInfo};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Loader for templates from the local filesystem.
pub struct LocalLoader<P: AsRef<std::path::Path>> {
    path: P,
}
impl<P: AsRef<std::path::Path>> LocalLoader<P> {
    /// Creates a new LocalLoader instance.
    pub fn new(path: P) -> Self {
        Self { path }
    }
}
impl<P: AsRef<std::path::Path>> TemplateLoader for LocalLoader<P> {
    /// Loads a template from the local filesystem.
    ///
    /// # Returns
    /// * `Result<LoadedTemplate>` - Loaded template with path and content hash
    fn load(&self) -> Result<LoadedTemplate> {
        let path = self.path.as_ref();
        if !path.exists() {
            return Err(Error::TemplateDoesNotExistsError {
                template_dir: path.display().to_string(),
            });
        }

        let root = path.to_path_buf();
        let hash = compute_directory_hash(&root)?;

        Ok(LoadedTemplate {
            root,
            source: TemplateSourceInfo::Filesystem {
                path: path.to_string_lossy().to_string(),
                hash,
            },
        })
    }
}

/// Compute a deterministic SHA-256 hash of all template files, excluding .bakerignore patterns.
///
/// File paths are collected and sorted for determinism. Both relative path and
/// file contents are fed into the hash so that renames are detected.
pub fn compute_directory_hash(template_root: &std::path::Path) -> Result<String> {
    let ignore_set = parse_bakerignore_file(template_root)?;
    let mut hasher = Sha256::new();

    let mut paths: Vec<PathBuf> = WalkDir::new(template_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| !ignore_set.is_match(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect();

    paths.sort();

    for file_path in paths {
        if let Ok(rel) = file_path.strip_prefix(template_root) {
            hasher.update(rel.to_string_lossy().as_bytes());
            hasher.update(b"\0");
        }
        let contents = std::fs::read(&file_path)?;
        hasher.update(&contents);
        hasher.update(b"\0");
    }

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn local_loader_returns_error_if_not_exists() {
        let loader = LocalLoader::new("does_not_exist_at_all");
        let result = loader.load();
        assert!(result.is_err());
        if let Err(Error::TemplateDoesNotExistsError { template_dir }) = result {
            assert_eq!(template_dir, "does_not_exist_at_all");
        } else {
            panic!("Expected TemplateDoesNotExistsError");
        }
    }

    #[test]
    fn compute_directory_hash_and_load_success() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("file1.txt"), b"hello").unwrap();
        fs::write(root.join("file2.txt"), b"world").unwrap();

        let hash1 = compute_directory_hash(root).unwrap();

        let loader = LocalLoader::new(root);
        let loaded = loader.load().unwrap();

        assert_eq!(loaded.root, root.to_path_buf());
        if let TemplateSourceInfo::Filesystem { path, hash } = loaded.source {
            assert_eq!(hash, hash1);
            assert_eq!(path, root.to_string_lossy().to_string());
        } else {
            panic!("Expected TemplateSourceInfo::Filesystem");
        }

        // Changing content should change the hash
        fs::write(root.join("file1.txt"), b"hello2").unwrap();
        let hash2 = compute_directory_hash(root).unwrap();
        assert_ne!(hash1, hash2);
    }
}
