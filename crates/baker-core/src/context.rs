//! Generation context for template processing.

use crate::{config::ConfigV1, types::SkipConfirm};
use std::path::PathBuf;

/// Shared state describing a single generation run.
pub struct GenerationContext {
    template_root: PathBuf,
    output_root: PathBuf,
    config: ConfigV1,
    answers: Option<serde_json::Value>,
    skip_confirms: Vec<SkipConfirm>,
    dry_run: bool,
}

impl GenerationContext {
    pub fn new(
        template_root: PathBuf,
        output_root: PathBuf,
        config: ConfigV1,
        skip_confirms: Vec<SkipConfirm>,
        dry_run: bool,
    ) -> Self {
        Self { template_root, output_root, config, answers: None, skip_confirms, dry_run }
    }

    pub fn template_root(&self) -> &PathBuf {
        &self.template_root
    }

    pub fn output_root(&self) -> &PathBuf {
        &self.output_root
    }

    pub fn config(&self) -> &ConfigV1 {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut ConfigV1 {
        &mut self.config
    }

    pub fn skip_confirms(&self) -> &[SkipConfirm] {
        &self.skip_confirms
    }

    pub fn dry_run(&self) -> bool {
        self.dry_run
    }

    pub fn set_answers(&mut self, answers: serde_json::Value) {
        self.answers = Some(answers);
    }

    pub fn answers(&self) -> &serde_json::Value {
        self.answers.as_ref().expect("generation answers requested before initialization")
    }

    pub fn answers_opt(&self) -> Option<&serde_json::Value> {
        self.answers.as_ref()
    }
}
