//! Common types used across the Baker crate.

use std::fmt::Display;

/// Skip confirmation prompts for specific stages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkipConfirm {
    /// Skip every confirmation prompt.
    All,
    /// Skip file overwrite confirmations.
    Overwrite,
    /// Skip hook execution confirmations.
    Hooks,
}

impl Display for SkipConfirm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SkipConfirm::All => "all",
            SkipConfirm::Overwrite => "overwrite",
            SkipConfirm::Hooks => "hooks",
        };
        write!(f, "{s}")
    }
}
