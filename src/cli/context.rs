use crate::{cli::SkipConfirm, config::ConfigV1};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigV1;
    use indexmap::IndexMap;
    use std::path::PathBuf;

    fn create_test_config() -> ConfigV1 {
        ConfigV1 {
            template_suffix: ".baker.j2".into(),
            loop_separator: "".into(),
            loop_content_separator: "".into(),
            template_globs: Vec::new(),
            import_root: None,
            questions: IndexMap::new(),
            post_hook_filename: "post".into(),
            pre_hook_filename: "pre".into(),
            post_hook_runner: Vec::new(),
            pre_hook_runner: Vec::new(),
            follow_symlinks: false,
        }
    }

    #[test]
    fn test_generation_context_new() {
        let ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        assert_eq!(ctx.template_root(), &PathBuf::from("/template"));
        assert_eq!(ctx.output_root(), &PathBuf::from("/output"));
        assert!(!ctx.dry_run());
    }

    #[test]
    fn test_generation_context_with_skip_confirms() {
        let ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![SkipConfirm::All, SkipConfirm::Overwrite],
            false,
        );
        assert!(ctx.skip_confirms().contains(&SkipConfirm::All));
        assert!(ctx.skip_confirms().contains(&SkipConfirm::Overwrite));
    }

    #[test]
    fn test_generation_context_dry_run() {
        let ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            true,
        );
        assert!(ctx.dry_run());
    }

    #[test]
    fn test_generation_context_config() {
        let ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        assert_eq!(ctx.config().template_suffix, ".baker.j2");
    }

    #[test]
    fn test_generation_context_config_mut() {
        let mut ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        ctx.config_mut().template_suffix = ".custom.j2".into();
        assert_eq!(ctx.config().template_suffix, ".custom.j2");
    }

    #[test]
    fn test_generation_context_answers_opt_none() {
        let ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        assert!(ctx.answers_opt().is_none());
    }

    #[test]
    fn test_generation_context_set_answers() {
        let mut ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        ctx.set_answers(serde_json::json!({"name": "test"}));
        assert!(ctx.answers_opt().is_some());
        assert_eq!(ctx.answers(), &serde_json::json!({"name": "test"}));
    }

    #[test]
    fn test_generation_context_answers_opt_some() {
        let mut ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        ctx.set_answers(serde_json::json!({"key": "value"}));
        let answers = ctx.answers_opt();
        assert!(answers.is_some());
        assert_eq!(answers.unwrap(), &serde_json::json!({"key": "value"}));
    }

    #[test]
    #[should_panic(expected = "generation answers requested before initialization")]
    fn test_generation_context_answers_panics_without_init() {
        let ctx = GenerationContext::new(
            PathBuf::from("/template"),
            PathBuf::from("/output"),
            create_test_config(),
            vec![],
            false,
        );
        let _ = ctx.answers();
    }
}
