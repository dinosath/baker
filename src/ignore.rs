use crate::error::Result;
use ignore::overrides::OverrideBuilder;
use log::{debug, info};
use std::path::Path;

/// Default patterns to always ignore during template processing
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
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

/// Builds an `Override` matcher from DEFAULT_IGNORE_PATTERNS for use with WalkBuilder.
/// This uses the ignore crate's native API for pattern matching with high priority.
/// The .bakerignore file should be registered on WalkBuilder via
/// `add_custom_ignore_filename(IGNORE_FILE)` and is applied with lower priority.
pub fn build_ignore_overrides<P: AsRef<Path>>(
    template_root: P,
) -> Result<ignore::overrides::Override> {
    let mut builder = OverrideBuilder::new(template_root);

    // OverrideBuilder patterns are allow-list globs by default.
    // Prefix with '!' to exclude/ignore these paths with high priority.
    debug!("Adding DEFAULT_IGNORE_PATTERNS as overrides: {:?}", DEFAULT_IGNORE_PATTERNS);
    for pattern in DEFAULT_IGNORE_PATTERNS {
        builder.add(&format!("!{pattern}"))?;
    }

    info!("Built ignore overrides from DEFAULT_IGNORE_PATTERNS");
    Ok(builder.build()?)
}
