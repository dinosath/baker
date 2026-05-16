//! Conflict marker types and utilities for merging file content.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Style to use when writing conflict markers into a file.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default,
)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
pub enum ConflictStyle {
    /// Git-style markers: `<<<<<<< current`, `=======`, `>>>>>>> updated`
    #[default]
    Git,
}

impl fmt::Display for ConflictStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictStyle::Git => write!(f, "git"),
        }
    }
}

/// Merge `existing` (current on-disk) and `updated` (newly rendered) content by
/// inserting conflict markers around every section that differs.
///
/// Diffing is performed line-by-line.  Identical leading and trailing lines are
/// kept as-is; only the changed region is wrapped in markers.
pub fn apply_conflict_markers(
    existing: &str,
    updated: &str,
    style: ConflictStyle,
) -> String {
    match style {
        ConflictStyle::Git => apply_git_style(existing, updated),
    }
}

/// Applies git-style conflict markers.
///
/// Finds the common prefix and suffix lines, then wraps the differing
/// middle section in `<<<<<<< current` / `=======` / `>>>>>>> updated` markers.
/// Trailing newline behaviour is preserved from the inputs.
fn apply_git_style(existing: &str, updated: &str) -> String {
    let old_lines: Vec<&str> = existing.lines().collect();
    let new_lines: Vec<&str> = updated.lines().collect();

    let prefix_len =
        old_lines.iter().zip(new_lines.iter()).take_while(|(a, b)| a == b).count();

    // Find common suffix length (must not overlap with prefix)
    let old_tail = &old_lines[prefix_len..];
    let new_tail = &new_lines[prefix_len..];
    let suffix_len = old_tail
        .iter()
        .rev()
        .zip(new_tail.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let old_mid = &old_lines[prefix_len..old_lines.len() - suffix_len];
    let new_mid = &new_lines[prefix_len..new_lines.len() - suffix_len];

    let mut result = String::new();

    for line in &old_lines[..prefix_len] {
        result.push_str(line);
        result.push('\n');
    }

    if old_mid != new_mid {
        result.push_str("<<<<<<< current\n");
        for line in old_mid {
            result.push_str(line);
            result.push('\n');
        }
        result.push_str("=======\n");
        for line in new_mid {
            result.push_str(line);
            result.push('\n');
        }
        result.push_str(">>>>>>> updated\n");
    }

    let suffix_start = old_lines.len() - suffix_len;
    for line in &old_lines[suffix_start..] {
        result.push_str(line);
        result.push('\n');
    }

    let existing_ends_newline = existing.ends_with('\n');
    let updated_ends_newline = updated.ends_with('\n');
    if !existing_ends_newline && !updated_ends_newline && result.ends_with('\n') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_content_produces_no_markers() {
        let content = "line1\nline2\nline3\n";
        let result = apply_conflict_markers(content, content, ConflictStyle::Git);
        assert_eq!(result, content);
        assert!(!result.contains("<<<<<<<"));
    }

    #[test]
    fn fully_different_content_wrapped_in_markers() {
        let existing = "old content\n";
        let updated = "new content\n";
        let result = apply_conflict_markers(existing, updated, ConflictStyle::Git);
        assert!(result.contains("<<<<<<< current\n"));
        assert!(result.contains("=======\n"));
        assert!(result.contains(">>>>>>> updated\n"));
        assert!(result.contains("old content\n"));
        assert!(result.contains("new content\n"));
    }

    #[test]
    fn common_prefix_and_suffix_preserved() {
        let existing = "header\nold line\nfooter\n";
        let updated = "header\nnew line\nfooter\n";
        let result = apply_conflict_markers(existing, updated, ConflictStyle::Git);
        let marker_start = result.find("<<<<<<<").unwrap();
        let marker_end = result.find(">>>>>>>").unwrap();
        let before = &result[..marker_start];
        let after = &result[marker_end + ">>>>>>> updated\n".len()..];
        assert!(before.contains("header"));
        assert!(after.contains("footer"));
    }

    #[test]
    fn conflict_style_default_is_git() {
        assert_eq!(ConflictStyle::default(), ConflictStyle::Git);
    }
}
