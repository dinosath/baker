//! Shared data models

use crate::config::BakerConfig;
use indexmap::IndexMap;
use serde_json::Value;

/// A template entry in the community or custom registry
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateEntry {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    /// GitHub owner / GitLab namespace
    pub owner: String,
    pub repo: String,
    pub branch: String,
    /// Sub-path within the repo where baker.yaml lives (default = "")
    pub path: String,
}

impl TemplateEntry {
    /// Raw GitHub base URL for file content
    pub fn raw_base_url(&self) -> String {
        let path = if self.path.is_empty() {
            String::new()
        } else {
            format!("{}/", self.path.trim_matches('/'))
        };
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.owner, self.repo, self.branch, path
        )
    }

    /// GitHub API for the tree rooted at `self.path`
    pub fn tree_api_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
            self.owner, self.repo, self.branch
        )
    }

    /// Human-readable slug for the project name default
    pub fn slug(&self) -> String {
        self.repo.clone()
    }
}

/// Application loading / error states
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Loadable<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Loadable<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Loadable::Loading)
    }
    pub fn is_failed(&self) -> bool {
        matches!(self, Loadable::Failed(_))
    }
    pub fn ready(self) -> Option<T> {
        match self {
            Loadable::Ready(v) => Some(v),
            _ => None,
        }
    }
    pub fn err_msg(&self) -> Option<&str> {
        match self {
            Loadable::Failed(m) => Some(m.as_str()),
            _ => None,
        }
    }
}

/// All global application state — kept at the root, passed as Signal<T>
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub active_tab: Tab,
    pub search_query: String,
    pub custom_url_input: String,
    pub selected_template: Option<TemplateEntry>,
    pub config_load: Loadable<BakerConfig>,
    pub form_values: IndexMap<String, Value>,
    pub output_files: Option<IndexMap<String, String>>,
    pub selected_preview_file: Option<String>,
    pub generating: bool,
    pub custom_templates: Vec<TemplateEntry>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Community,
            search_query: String::new(),
            custom_url_input: String::new(),
            selected_template: None,
            config_load: Loadable::Idle,
            form_values: IndexMap::new(),
            output_files: None,
            selected_preview_file: None,
            generating: false,
            custom_templates: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Community,
    Custom,
}
