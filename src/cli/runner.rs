use crate::{
    cli::{answers::AnswerCollector, processor::FileProcessor, Args, SkipConfirm},
    config::Config,
    error::Result,
    hooks::{confirm_hook_execution, get_hook_files, run_hook},
    ignore::parse_bakerignore_file,
    import::add_templates_in_renderer,
    ioutils::get_output_dir,
    loader::get_template,
    template::{get_template_engine, processor::TemplateProcessor},
};
use serde_json::json;
use std::path::PathBuf;

/// Main CLI runner that orchestrates the entire template generation workflow
pub struct Runner {
    args: Args,
}

impl Runner {
    pub fn new(args: Args) -> Self {
        Self { args }
    }

    /// Executes the complete template generation workflow
    pub fn run(self) -> Result<()> {
        let mut engine = get_template_engine();

        let output_root = get_output_dir(&self.args.output_dir, self.args.force)?;

        let template_root = get_template(
            self.args.template.as_str(),
            self.should_skip_overwrite_prompts(),
        )?;

        let config = self.load_and_validate_config(&template_root)?;

        add_templates_in_renderer(&template_root, &config, &mut engine);

        let (pre_hook_file, post_hook_file, execute_hooks) =
            self.setup_hooks(&template_root, &config, &engine)?;

        // Execute pre-generation hook
        let pre_hook_output = self.execute_pre_hook(&template_root, &output_root, &pre_hook_file, execute_hooks)?;

        // Collect answers from all sources
        let answers = self.collect_answers(&config, &engine, pre_hook_output)?;

        // Process template files
        self.process_template_files(&template_root, &output_root, &config, &engine, &answers)?;

        // Execute post-generation hook
        self.execute_post_hook(&template_root, &output_root, &post_hook_file, &answers, execute_hooks)?;

        println!(
            "Template generation completed successfully in {}.",
            output_root.display()
        );
        Ok(())
    }

    /// Loads and validates the template configuration
    fn load_and_validate_config(&self, template_root: &PathBuf) -> Result<crate::config::ConfigV1> {
        let config = Config::load_config(template_root)?;
        let Config::V1(config) = config;
        config.validate()?;
        Ok(config)
    }

    /// Sets up pre and post hook files
    fn setup_hooks(
        &self,
        template_root: &PathBuf,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
    ) -> Result<(PathBuf, PathBuf, bool)> {
        let pre_hook_filename = engine.render(
            &config.pre_hook_filename,
            &json!({}),
            Some(&config.pre_hook_filename),
        )?;
        let post_hook_filename = engine.render(
            &config.post_hook_filename,
            &json!({}),
            Some(&config.post_hook_filename),
        )?;

        let execute_hooks = confirm_hook_execution(
            template_root,
            self.should_skip_hook_prompts(),
            &pre_hook_filename,
            &post_hook_filename,
        )?;

        let (pre_hook_file, post_hook_file) =
            get_hook_files(template_root, &pre_hook_filename, &post_hook_filename);

        Ok((pre_hook_file, post_hook_file, execute_hooks))
    }

    /// Executes the pre-generation hook if it exists
    fn execute_pre_hook(
        &self,
        template_root: &PathBuf,
        output_root: &PathBuf,
        pre_hook_file: &PathBuf,
        execute_hooks: bool,
    ) -> Result<Option<String>> {
        if execute_hooks && pre_hook_file.exists() {
            log::debug!("Executing pre-hook: {}", pre_hook_file.display());
            run_hook(template_root, output_root, pre_hook_file, None)
        } else {
            Ok(None)
        }
    }

    /// Collects answers from all available sources
    fn collect_answers(
        &self,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
        pre_hook_output: Option<String>,
    ) -> Result<serde_json::Value> {
        let collector = AnswerCollector::new(engine, self.args.non_interactive);
        collector.collect_answers(config, pre_hook_output, self.args.answers.clone())
    }

    /// Processes all template files
    fn process_template_files(
        &self,
        template_root: &PathBuf,
        output_root: &PathBuf,
        config: &crate::config::ConfigV1,
        engine: &dyn crate::renderer::TemplateRenderer,
        answers: &serde_json::Value,
    ) -> Result<()> {
        let bakerignore = parse_bakerignore_file(template_root)?;

        let processor = TemplateProcessor::new(
            engine,
            template_root.clone(),
            output_root.clone(),
            answers,
            &bakerignore,
            config.template_suffix.as_str(),
        );

        let file_processor = FileProcessor::new(processor, &self.args.skip_confirms);
        file_processor.process_all_files(template_root)
    }

    /// Executes the post-generation hook if it exists
    fn execute_post_hook(
        &self,
        template_root: &PathBuf,
        output_root: &PathBuf,
        post_hook_file: &PathBuf,
        answers: &serde_json::Value,
        execute_hooks: bool,
    ) -> Result<()> {
        if execute_hooks && post_hook_file.exists() {
            log::debug!("Executing post-hook: {}", post_hook_file.display());
            let post_hook_stdout =
                run_hook(template_root, output_root, post_hook_file, Some(answers))?;

            if let Some(result) = post_hook_stdout {
                log::debug!("Post-hook stdout content: {result}");
            }
        }
        Ok(())
    }

    /// Determines if overwrite prompts should be skipped
    fn should_skip_overwrite_prompts(&self) -> bool {
        self.args.skip_confirms.contains(&SkipConfirm::All)
            || self.args.skip_confirms.contains(&SkipConfirm::Overwrite)
    }

    /// Determines if hook execution prompts should be skipped
    fn should_skip_hook_prompts(&self) -> bool {
        self.args.skip_confirms.contains(&SkipConfirm::All)
            || self.args.skip_confirms.contains(&SkipConfirm::Hooks)
    }
}

/// Main entry point for CLI execution
pub fn run(args: Args) -> Result<()> {
    let runner = Runner::new(args);
    runner.run()
}
