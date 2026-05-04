use crate::error::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use log::{debug, info};
use std::{fs::read_to_string, path::Path};

/// Default patterns to always ignore during template processing
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git/**",
    ".git",
    ".hg/**",
    ".hg",
    ".svn/**",
    ".svn",
    "**/.DS_Store",
    ".bakerignore",
    "hooks",
    "hooks/**",
    "baker.yaml",
    "baker.yml",
    "baker.json",
];

/// Baker's ignore file name
pub const IGNORE_FILE: &str = ".bakerignore";

/// Reads and processes the .bakerignore file to create a set of glob patterns.
pub fn parse_bakerignore_file<P: AsRef<Path>>(template_root: P) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let template_root = template_root.as_ref();
    let bakerignore_path = template_root.join(IGNORE_FILE);

    // Add default patterns first
    let mut patterns: Vec<String> = DEFAULT_IGNORE_PATTERNS
        .iter()
        .map(|pattern| {
            let path_to_ignored_pattern = template_root.join(pattern);
            path_to_ignored_pattern.to_string_lossy().to_string()
        })
        .collect();

    // Then add patterns from .bakerignore if it exists
    if let Ok(contents) = read_to_string(bakerignore_path) {
        let ignored_patterns: Vec<String> = contents
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let path_to_ignored_pattern = template_root.join(line);
                path_to_ignored_pattern.to_string_lossy().to_string()
            })
            .collect();
        patterns.extend(ignored_patterns);
    } else {
        debug!("No .bakerignore file found, using default patterns.");
    }

    for pattern in &patterns {
        debug!("Adding ignore pattern: {pattern} to globset");
        builder.add(Glob::new(pattern)?);
    }
    info!("Loaded the following ignore patterns from .bakerignore file: {patterns:?}");
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn parse_bakerignore_file_adds_default_patterns() {
        let dir = tempdir().unwrap();
        let globset = parse_bakerignore_file(dir.path()).unwrap();

        let git_dir = dir.path().join(".git");
        let ds_store = dir.path().join("foo/.DS_Store");
        let baker_yaml = dir.path().join("baker.yaml");
        let baker_yml = dir.path().join("baker.yml");
        let bakerignore = dir.path().join(".bakerignore");
        let unignored = dir.path().join("src/main.rs");

        assert!(globset.is_match(&git_dir));
        assert!(globset.is_match(&ds_store));
        assert!(globset.is_match(&baker_yaml));
        assert!(globset.is_match(&baker_yml));
        assert!(globset.is_match(&bakerignore));
        assert!(!globset.is_match(&unignored));
    }

    #[test]
    fn parse_bakerignore_file_adds_custom_patterns() {
        let dir = tempdir().unwrap();
        let ignore_file_path = dir.path().join(IGNORE_FILE);

        let mut file = File::create(ignore_file_path).unwrap();
        writeln!(file, "target/").unwrap();
        writeln!(file, "*.tmp").unwrap();
        writeln!(file, "# A comment").unwrap();
        writeln!(file, "   ").unwrap(); // blank line
        writeln!(file, "secret.key").unwrap();

        let globset = parse_bakerignore_file(dir.path()).unwrap();

        let target_dir = dir.path().join("target/");
        let tmp_file = dir.path().join("test.tmp");
        let secret_file = dir.path().join("secret.key");
        let normal_file = dir.path().join("normal.txt");
        let baker_yaml = dir.path().join("baker.yaml"); // Default pattern should still exist

        assert!(globset.is_match(&target_dir));
        assert!(globset.is_match(&tmp_file));
        assert!(globset.is_match(&secret_file));
        assert!(!globset.is_match(&normal_file));
        assert!(globset.is_match(&baker_yaml));
    }
}
