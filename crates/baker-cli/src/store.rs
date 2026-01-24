//! Template store for managing installed templates.
//!
//! This module provides functionality to install, list, remove, and use
//! templates from a local store. Templates are stored in compressed format
//! and decompressed temporarily when used.

use baker_core::error::{Error, Result};
use baker_core::loader::get_template;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use walkdir::WalkDir;

/// Metadata about an installed template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Name of the template (used as identifier).
    pub name: String,
    /// Original source (git URL or local path).
    pub source: String,
    /// Installation timestamp.
    pub installed_at: String,
    /// Optional description.
    pub description: Option<String>,
}

/// Manages the local template store.
pub struct TemplateStore {
    /// Root directory for the template store.
    store_dir: PathBuf,
}

impl TemplateStore {
    /// Creates a new TemplateStore using the default data directory.
    ///
    /// The store is located at:
    /// - Linux: `~/.local/share/baker/templates`
    /// - macOS: `~/Library/Application Support/baker/templates`
    /// - Windows: `C:\Users\<User>\AppData\Roaming\baker\templates`
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir().ok_or_else(|| {
            Error::Other(anyhow::anyhow!("Could not determine data directory"))
        })?;
        let store_dir = data_dir.join("baker").join("templates");
        Ok(Self { store_dir })
    }

    /// Creates a TemplateStore with a custom directory (useful for testing).
    pub fn with_dir(store_dir: PathBuf) -> Self {
        Self { store_dir }
    }

    /// Returns the path to the store directory.
    pub fn store_dir(&self) -> &Path {
        &self.store_dir
    }

    /// Ensures the store directory exists.
    fn ensure_store_dir(&self) -> Result<()> {
        if !self.store_dir.exists() {
            fs::create_dir_all(&self.store_dir).map_err(|e| {
                Error::Other(anyhow::anyhow!(
                    "Failed to create store directory '{}': {}",
                    self.store_dir.display(),
                    e
                ))
            })?;
            debug!("Created template store at: {}", self.store_dir.display());
        }
        Ok(())
    }

    /// Returns the path to a template's compressed archive.
    fn template_archive_path(&self, name: &str) -> PathBuf {
        self.store_dir.join(format!("{name}.tar.gz"))
    }

    /// Returns the path to a template's metadata file.
    fn template_metadata_path(&self, name: &str) -> PathBuf {
        self.store_dir.join(format!("{name}.json"))
    }

    /// Installs a template from a git repository or local path.
    ///
    /// # Arguments
    /// * `source` - Git URL or local path to the template
    /// * `name` - Name to use for the installed template
    /// * `description` - Optional description
    /// * `force` - Whether to overwrite an existing template
    pub fn install(
        &self,
        source: &str,
        name: &str,
        description: Option<String>,
        force: bool,
    ) -> Result<()> {
        self.ensure_store_dir()?;

        let archive_path = self.template_archive_path(name);
        let metadata_path = self.template_metadata_path(name);

        // Check if template already exists
        if archive_path.exists() && !force {
            return Err(Error::Other(anyhow::anyhow!(
                "Template '{}' already exists. Use --force to overwrite.",
                name
            )));
        }

        info!("Fetching template from: {}", source);

        // Load template from source (handles both git and local)
        let template_path = get_template(source, true)?;

        info!("Compressing template...");

        // Create compressed archive
        self.compress_directory(&template_path, &archive_path)?;

        // Save metadata
        let metadata = TemplateMetadata {
            name: name.to_string(),
            source: source.to_string(),
            installed_at: chrono_lite_timestamp(),
            description,
        };
        self.save_metadata(&metadata_path, &metadata)?;

        info!(
            "Template '{}' installed successfully ({} bytes)",
            name,
            fs::metadata(&archive_path).map(|m| m.len()).unwrap_or(0)
        );

        Ok(())
    }

    /// Compresses a directory into a tar.gz archive.
    fn compress_directory(&self, source_dir: &Path, archive_path: &Path) -> Result<()> {
        let file = File::create(archive_path).map_err(|e| {
            Error::Other(anyhow::anyhow!(
                "Failed to create archive '{}': {}",
                archive_path.display(),
                e
            ))
        })?;

        let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        let mut archive = Builder::new(encoder);

        for entry in WalkDir::new(source_dir) {
            let entry = entry.map_err(|e| {
                Error::Other(anyhow::anyhow!("Failed to walk directory: {}", e))
            })?;

            let path = entry.path();
            let relative_path = path.strip_prefix(source_dir).map_err(|e| {
                Error::Other(anyhow::anyhow!("Failed to get relative path: {}", e))
            })?;

            // Skip the root directory itself
            if relative_path.as_os_str().is_empty() {
                continue;
            }

            // Skip .git directory
            if relative_path.components().any(|c| c.as_os_str() == ".git") {
                continue;
            }

            if path.is_file() {
                debug!("Adding file: {}", relative_path.display());
                archive.append_path_with_name(path, relative_path).map_err(|e| {
                    Error::Other(anyhow::anyhow!(
                        "Failed to add '{}' to archive: {}",
                        relative_path.display(),
                        e
                    ))
                })?;
            } else if path.is_dir() {
                debug!("Adding directory: {}", relative_path.display());
                archive.append_dir(relative_path, path).map_err(|e| {
                    Error::Other(anyhow::anyhow!(
                        "Failed to add directory '{}' to archive: {}",
                        relative_path.display(),
                        e
                    ))
                })?;
            }
        }

        archive.finish().map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to finish archive: {}", e))
        })?;

        Ok(())
    }

    /// Saves template metadata to a JSON file.
    fn save_metadata(&self, path: &Path, metadata: &TemplateMetadata) -> Result<()> {
        let file = File::create(path).map_err(|e| {
            Error::Other(anyhow::anyhow!(
                "Failed to create metadata file '{}': {}",
                path.display(),
                e
            ))
        })?;
        serde_json::to_writer_pretty(file, metadata).map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to write metadata: {}", e))
        })?;
        Ok(())
    }

    /// Loads template metadata from a JSON file.
    fn load_metadata(&self, path: &Path) -> Result<TemplateMetadata> {
        let file = File::open(path).map_err(|e| {
            Error::Other(anyhow::anyhow!(
                "Failed to open metadata file '{}': {}",
                path.display(),
                e
            ))
        })?;
        serde_json::from_reader(file)
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to parse metadata: {}", e)))
    }

    /// Lists all installed templates.
    pub fn list(&self) -> Result<Vec<TemplateMetadata>> {
        if !self.store_dir.exists() {
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();
        for entry in fs::read_dir(&self.store_dir).map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to read store directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                Error::Other(anyhow::anyhow!("Failed to read entry: {}", e))
            })?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                match self.load_metadata(&path) {
                    Ok(metadata) => templates.push(metadata),
                    Err(e) => {
                        debug!(
                            "Skipping invalid metadata file '{}': {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        templates.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(templates)
    }

    /// Removes an installed template.
    pub fn remove(&self, name: &str) -> Result<()> {
        let archive_path = self.template_archive_path(name);
        let metadata_path = self.template_metadata_path(name);

        if !archive_path.exists() {
            return Err(Error::Other(anyhow::anyhow!(
                "Template '{}' is not installed",
                name
            )));
        }

        fs::remove_file(&archive_path).map_err(|e| {
            Error::Other(anyhow::anyhow!(
                "Failed to remove archive '{}': {}",
                archive_path.display(),
                e
            ))
        })?;

        if metadata_path.exists() {
            fs::remove_file(&metadata_path).map_err(|e| {
                Error::Other(anyhow::anyhow!(
                    "Failed to remove metadata '{}': {}",
                    metadata_path.display(),
                    e
                ))
            })?;
        }

        info!("Template '{}' removed successfully", name);
        Ok(())
    }

    /// Checks if a template is installed.
    pub fn is_installed(&self, name: &str) -> bool {
        self.template_archive_path(name).exists()
    }

    /// Extracts a template to a temporary directory and returns the path.
    ///
    /// The caller is responsible for cleaning up the temporary directory.
    pub fn extract_to_temp(&self, name: &str) -> Result<tempfile::TempDir> {
        let archive_path = self.template_archive_path(name);

        if !archive_path.exists() {
            return Err(Error::Other(anyhow::anyhow!(
                "Template '{}' is not installed",
                name
            )));
        }

        let temp_dir = tempfile::tempdir().map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to create temporary directory: {}", e))
        })?;

        debug!("Extracting template '{}' to: {}", name, temp_dir.path().display());

        let file = File::open(&archive_path).map_err(|e| {
            Error::Other(anyhow::anyhow!(
                "Failed to open archive '{}': {}",
                archive_path.display(),
                e
            ))
        })?;

        let decoder = GzDecoder::new(BufReader::new(file));
        let mut archive = Archive::new(decoder);

        archive.unpack(temp_dir.path()).map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to extract template '{}': {}", name, e))
        })?;

        Ok(temp_dir)
    }

    /// Gets metadata for a specific template.
    pub fn get_metadata(&self, name: &str) -> Result<TemplateMetadata> {
        let metadata_path = self.template_metadata_path(name);
        if !metadata_path.exists() {
            return Err(Error::Other(anyhow::anyhow!(
                "Template '{}' is not installed",
                name
            )));
        }
        self.load_metadata(&metadata_path)
    }
}

/// Returns a simple ISO 8601 timestamp without external dependencies.
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // Simple calculation for UTC time
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let mut remaining_days = days as i64;
    let mut year = 1970i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for days in days_in_months.iter() {
        if remaining_days < *days {
            break;
        }
        remaining_days -= *days;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_store_install_and_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        // Create a simple template directory
        let template_dir = temp_dir.path().join("my_template");
        fs::create_dir_all(&template_dir).unwrap();
        fs::write(template_dir.join("baker.yaml"), "version: 1\nquestions: {}").unwrap();
        fs::write(template_dir.join("README.md"), "# Test Template").unwrap();

        // Install the template
        store
            .install(
                template_dir.to_str().unwrap(),
                "test-template",
                Some("A test template".to_string()),
                false,
            )
            .unwrap();

        // Verify it's installed
        assert!(store.is_installed("test-template"));

        // List templates
        let templates = store.list().unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "test-template");
        assert_eq!(templates[0].description, Some("A test template".to_string()));

        // Extract and verify
        let extracted = store.extract_to_temp("test-template").unwrap();
        assert!(extracted.path().join("baker.yaml").exists());
        assert!(extracted.path().join("README.md").exists());

        // Remove template
        store.remove("test-template").unwrap();
        assert!(!store.is_installed("test-template"));
    }

    #[test]
    fn test_install_local_template_with_nested_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        // Create a template with nested directories
        let template_dir = temp_dir.path().join("nested_template");
        fs::create_dir_all(template_dir.join("src/components")).unwrap();
        fs::create_dir_all(template_dir.join("tests")).unwrap();

        fs::write(
            template_dir.join("baker.yaml"),
            "schemaVersion: v1\nquestions:\n  name:\n    type: str\n    default: test",
        )
        .unwrap();
        fs::write(template_dir.join("README.md.baker.j2"), "# {{ name }}").unwrap();
        fs::write(template_dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(template_dir.join("src/components/button.rs"), "pub struct Button;")
            .unwrap();
        fs::write(template_dir.join("tests/test_main.rs"), "#[test] fn test() {}")
            .unwrap();

        // Install
        store.install(template_dir.to_str().unwrap(), "nested", None, false).unwrap();

        // Extract and verify structure
        let extracted = store.extract_to_temp("nested").unwrap();
        assert!(extracted.path().join("baker.yaml").exists());
        assert!(extracted.path().join("README.md.baker.j2").exists());
        assert!(extracted.path().join("src/main.rs").exists());
        assert!(extracted.path().join("src/components/button.rs").exists());
        assert!(extracted.path().join("tests/test_main.rs").exists());

        // Verify content
        let button_content =
            fs::read_to_string(extracted.path().join("src/components/button.rs"))
                .unwrap();
        assert_eq!(button_content, "pub struct Button;");
    }

    #[test]
    fn test_install_fails_without_force_when_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        let template_dir = temp_dir.path().join("template");
        fs::create_dir_all(&template_dir).unwrap();
        fs::write(template_dir.join("baker.yaml"), "version: 1\nquestions: {}").unwrap();

        // First install should succeed
        store.install(template_dir.to_str().unwrap(), "dup-test", None, false).unwrap();

        // Second install without force should fail
        let result =
            store.install(template_dir.to_str().unwrap(), "dup-test", None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_install_succeeds_with_force_when_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        let template_dir = temp_dir.path().join("template");
        fs::create_dir_all(&template_dir).unwrap();
        fs::write(template_dir.join("baker.yaml"), "version: 1\nquestions: {}").unwrap();
        fs::write(template_dir.join("file.txt"), "original").unwrap();

        // First install
        store
            .install(
                template_dir.to_str().unwrap(),
                "force-test",
                Some("Original".to_string()),
                false,
            )
            .unwrap();

        // Modify the template
        fs::write(template_dir.join("file.txt"), "updated").unwrap();

        // Second install with force should succeed
        store
            .install(
                template_dir.to_str().unwrap(),
                "force-test",
                Some("Updated".to_string()),
                true,
            )
            .unwrap();

        // Verify update
        let metadata = store.get_metadata("force-test").unwrap();
        assert_eq!(metadata.description, Some("Updated".to_string()));

        let extracted = store.extract_to_temp("force-test").unwrap();
        let content = fs::read_to_string(extracted.path().join("file.txt")).unwrap();
        assert_eq!(content, "updated");
    }

    #[test]
    fn test_remove_nonexistent_template_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        let result = store.remove("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not installed"));
    }

    #[test]
    fn test_extract_nonexistent_template_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        let result = store.extract_to_temp("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not installed"));
    }

    #[test]
    fn test_list_empty_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        let templates = store.list().unwrap();
        assert!(templates.is_empty());
    }

    #[test]
    fn test_multiple_templates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        // Create and install multiple templates
        for i in 1..=3 {
            let template_dir = temp_dir.path().join(format!("template_{}", i));
            fs::create_dir_all(&template_dir).unwrap();
            fs::write(
                template_dir.join("baker.yaml"),
                format!("version: 1\nquestions: {{}}\n# Template {}", i),
            )
            .unwrap();

            store
                .install(
                    template_dir.to_str().unwrap(),
                    &format!("template-{}", i),
                    Some(format!("Template number {}", i)),
                    false,
                )
                .unwrap();
        }

        // Verify all installed
        let templates = store.list().unwrap();
        assert_eq!(templates.len(), 3);

        // Templates should be sorted by name
        assert_eq!(templates[0].name, "template-1");
        assert_eq!(templates[1].name, "template-2");
        assert_eq!(templates[2].name, "template-3");

        // Remove one
        store.remove("template-2").unwrap();

        let templates = store.list().unwrap();
        assert_eq!(templates.len(), 2);
        assert!(!store.is_installed("template-2"));
    }

    #[test]
    fn test_git_directory_excluded_from_archive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = TemplateStore::with_dir(temp_dir.path().join("store"));

        // Create a template with a .git directory (simulating a git repo)
        let template_dir = temp_dir.path().join("git_template");
        fs::create_dir_all(template_dir.join(".git/objects")).unwrap();
        fs::write(template_dir.join(".git/config"), "[core]\nbare = false").unwrap();
        fs::write(template_dir.join("baker.yaml"), "version: 1\nquestions: {}").unwrap();
        fs::write(template_dir.join("README.md"), "# Template").unwrap();

        // Install
        store.install(template_dir.to_str().unwrap(), "git-test", None, false).unwrap();

        // Extract and verify .git is excluded
        let extracted = store.extract_to_temp("git-test").unwrap();
        assert!(extracted.path().join("baker.yaml").exists());
        assert!(extracted.path().join("README.md").exists());
        assert!(!extracted.path().join(".git").exists());
    }

    #[test]
    fn test_chrono_lite_timestamp() {
        let ts = chrono_lite_timestamp();
        // Should be in ISO 8601 format
        assert!(ts.contains("T"));
        assert!(ts.ends_with("Z"));
        assert_eq!(ts.len(), 20);
    }
}
