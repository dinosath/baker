//! `baker update` — re-runs generation when the template has changed, using conflict
//! markers to highlight differences in files the user has modified.

use crate::{
    cli::{
        answers::AnswerCollector, context::GenerationContext, hooks::run_hook,
        processor::FileProcessor, UpdateArgs,
    },
    config::{Config, ConfigV1},
    conflict::ConflictStyle,
    error::Result,
    generated::{self, BakerGenerated},
    ignore::parse_bakerignore_file,
    loader::{get_template, TemplateSourceInfo},
    renderer::TemplateRenderer,
    template::{get_template_engine, processor::TemplateProcessor},
};
use globset::{Glob, GlobSetBuilder};
use serde_json::json;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Main entry point for `baker update`.
pub fn run_update(args: UpdateArgs) -> Result<()> {
    UpdateRunner::new(args).run()
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

struct UpdateRunner {
    args: UpdateArgs,
}

impl UpdateRunner {
    fn new(args: UpdateArgs) -> Self {
        Self { args }
    }

    fn run(self) -> Result<()> {
        // 1. Determine the generated-metadata filename.
        let file_name = self
            .args
            .generated_file
            .as_deref()
            .unwrap_or(crate::constants::DEFAULT_GENERATED_FILE_NAME);

        // 2. Read the .baker-generated.yaml from the current working directory.
        let cwd = std::env::current_dir()?;
        let meta: BakerGenerated = generated::read(&cwd, file_name)?;

        log::info!("Found generated metadata (generated_at={})", meta.generated_at);

        // 3. Re-fetch the template into a temp directory so we can compare.
        let skip_overwrite = self.should_skip_overwrite_prompts();
        let loaded = self.fetch_updated_template(&meta.template, skip_overwrite)?;

        // 4. Compare source hashes / commits to detect changes.
        if self.sources_are_identical(&meta.template, &loaded.source) {
            println!("Template has not changed since last generation — nothing to do.");
            return Ok(());
        }

        // 5. Merge answers: saved → CLI overrides.
        let merged_answers = self.merge_answers(meta.answers.clone())?;

        // 6. Load config from the freshly-fetched template.
        let config = load_and_validate_config(&loaded.root)?;

        // 7. Determine conflict style: CLI flag → config field → default.
        let conflict_style: Option<ConflictStyle> =
            self.args.conflict_style.or(config.conflict_marker_style);

        // 8. Build a GenerationContext in conflict mode.
        let mut context = GenerationContext::new(
            loaded.root.clone(),
            cwd.clone(),
            config,
            self.args.skip_confirms.clone(),
            self.args.dry_run,
            true, // conflict_mode
            conflict_style,
        );
        context.set_answers(merged_answers.clone());

        // 9. Build template engine and add import templates.
        let mut engine = get_template_engine();
        add_templates_in_renderer(&loaded.root, context.config(), &mut engine);

        // 10. Run pre-hook if present.
        let pre_hook_output = self.maybe_run_pre_hook(&context, &engine)?;

        // If the pre-hook produced new answers we merge them in (same logic as generate).
        if let Some(ref hook_json) = pre_hook_output {
            if let Ok(hook_val) = serde_json::from_str::<serde_json::Value>(hook_json) {
                if let (Some(base), Some(extra)) =
                    (merged_answers.as_object(), hook_val.as_object())
                {
                    let mut merged = base.clone();
                    merged.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));
                    context.set_answers(serde_json::Value::Object(merged));
                }
            }
        }

        // 11. Allow collecting missing/new questions introduced in the updated template.
        //
        // The saved + CLI-overridden answers are serialised back to a JSON string and
        // passed as `cli_answers` so that `AnswerCollector` uses them as its starting
        // base.  Any question that already has an answer will be skipped in non-
        // interactive mode; new questions added to the template since the last
        // generation will be prompted normally (or use their default in --non-interactive).
        let merged_json_str = serde_json::to_string(context.answers())?;
        let collector =
            AnswerCollector::new(&engine, self.args.non_interactive, &loaded.root);
        let final_answers = collector.collect_answers(
            context.config(),
            pre_hook_output,
            Some(merged_json_str),
            None,
        )?;
        context.set_answers(final_answers);

        // 12. Process templates (with conflict markers on differing files).
        let bakerignore = parse_bakerignore_file(context.template_root())?;
        let processor = TemplateProcessor::new(&engine, &context, &bakerignore);
        let file_processor = FileProcessor::new(processor, &context);
        file_processor.process_all_files()?;

        // 13. Run post-hook if present.
        self.maybe_run_post_hook(&context, &engine)?;

        // 14. Write updated .baker-generated.yaml.
        if context.dry_run() {
            log::info!(
                "[DRY RUN] Would write updated generated metadata to '{}'",
                cwd.join(file_name).display()
            );
        } else {
            let new_meta = BakerGenerated::new(loaded.source, context.answers().clone());
            generated::write(&cwd, file_name, &new_meta)?;
        }

        println!(
            "{}",
            if context.dry_run() {
                "[DRY RUN] Update complete (no files were modified)".to_string()
            } else {
                "Update complete. Check files for conflict markers (<<<<<<< current)."
                    .to_string()
            }
        );

        Ok(())
    }

    // -- Template re-fetching -----------------------------------------------

    fn fetch_updated_template(
        &self,
        stored: &TemplateSourceInfo,
        skip_overwrite: bool,
    ) -> Result<crate::loader::LoadedTemplate> {
        match stored {
            TemplateSourceInfo::Git { url, .. } => {
                // Clone fresh into a temp directory so we don't clobber the
                // working directory clone the user may have.
                let tmp = tempfile::TempDir::new()?;
                let tmp_path = tmp.path().to_path_buf();
                // We clone into a sub-directory so the GitLoader's repo-name
                // extraction works correctly.
                let loaded = clone_git_into_tmp(url, &tmp_path)?;
                // Keep the TempDir alive until we're done by leaking it — the
                // OS will clean it up at process exit.  This avoids lifetime
                // issues while keeping the code simple.
                std::mem::forget(tmp);
                Ok(loaded)
            }
            TemplateSourceInfo::Filesystem { path, .. } => {
                get_template(path.as_str(), skip_overwrite)
            }
        }
    }

    // -- Hash / commit comparison -------------------------------------------

    fn sources_are_identical(
        &self,
        stored: &TemplateSourceInfo,
        fresh: &TemplateSourceInfo,
    ) -> bool {
        match (stored, fresh) {
            (
                TemplateSourceInfo::Git { commit: old_commit, .. },
                TemplateSourceInfo::Git { commit: new_commit, .. },
            ) => old_commit == new_commit,
            (
                TemplateSourceInfo::Filesystem { hash: old_hash, .. },
                TemplateSourceInfo::Filesystem { hash: new_hash, .. },
            ) => old_hash == new_hash,
            _ => false,
        }
    }

    // -- Answer merging -----------------------------------------------------

    fn merge_answers(&self, saved: serde_json::Value) -> Result<serde_json::Value> {
        let mut base = match saved {
            serde_json::Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };

        // Apply --answers-file override
        if let Some(ref path) = self.args.answers_file {
            let content = std::fs::read_to_string(path)?;
            if let Ok(serde_json::Value::Object(extra)) =
                serde_json::from_str::<serde_json::Value>(&content)
            {
                base.extend(extra);
            }
        }

        // Apply --answers (inline JSON) override
        if let Some(ref answers_str) = self.args.answers {
            let raw = if answers_str == crate::constants::STDIN_INDICATOR {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
                buf
            } else {
                answers_str.clone()
            };
            if let Ok(serde_json::Value::Object(extra)) =
                serde_json::from_str::<serde_json::Value>(&raw)
            {
                base.extend(extra);
            }
        }

        Ok(serde_json::Value::Object(base))
    }

    // -- Determine skip flags -----------------------------------------------

    fn should_skip_overwrite_prompts(&self) -> bool {
        use crate::cli::SkipConfirm;
        self.args.skip_confirms.contains(&SkipConfirm::All)
            || self.args.skip_confirms.contains(&SkipConfirm::Overwrite)
    }

    fn should_skip_hook_prompts(&self) -> bool {
        use crate::cli::SkipConfirm;
        self.args.skip_confirms.contains(&SkipConfirm::All)
            || self.args.skip_confirms.contains(&SkipConfirm::Hooks)
    }

    // -- Hook helpers (minimal — mirrored from Runner) ----------------------

    fn maybe_run_pre_hook(
        &self,
        context: &GenerationContext,
        engine: &dyn TemplateRenderer,
    ) -> Result<Option<String>> {
        let config = context.config();
        let pre_hook_filename = engine
            .render(
                &config.pre_hook_filename,
                &json!({}),
                Some(&config.pre_hook_filename),
            )
            .unwrap_or_else(|_| config.pre_hook_filename.clone());

        let pre_hook_file =
            context.template_root().join("hooks").join(&pre_hook_filename);

        if !pre_hook_file.exists() {
            return Ok(None);
        }

        if context.dry_run() {
            log::info!("[DRY RUN] Would execute pre-hook: {}", pre_hook_file.display());
            return Ok(None);
        }

        let execute = self.confirm_hook_execution(
            context.template_root(),
            self.should_skip_hook_prompts(),
            &pre_hook_filename,
        )?;

        if execute {
            let runner = render_hook_runner(
                engine,
                &config.pre_hook_runner,
                context.answers_opt(),
            )?;
            run_hook(
                context.template_root(),
                context.output_root(),
                &pre_hook_file,
                None,
                &runner,
                false,
            )
        } else {
            Ok(None)
        }
    }

    fn maybe_run_post_hook(
        &self,
        context: &GenerationContext,
        engine: &dyn TemplateRenderer,
    ) -> Result<()> {
        let config = context.config();
        let post_hook_filename = engine
            .render(
                &config.post_hook_filename,
                &json!({}),
                Some(&config.post_hook_filename),
            )
            .unwrap_or_else(|_| config.post_hook_filename.clone());

        let post_hook_file =
            context.template_root().join("hooks").join(&post_hook_filename);

        if !post_hook_file.exists() {
            return Ok(());
        }

        if context.dry_run() {
            log::info!("[DRY RUN] Would execute post-hook: {}", post_hook_file.display());
            return Ok(());
        }

        let execute = self.confirm_hook_execution(
            context.template_root(),
            self.should_skip_hook_prompts(),
            &post_hook_filename,
        )?;

        if execute {
            let runner = render_hook_runner(
                engine,
                &config.post_hook_runner,
                context.answers_opt(),
            )?;
            run_hook(
                context.template_root(),
                context.output_root(),
                &post_hook_file,
                Some(context.answers()),
                &runner,
                config.post_hook_print_stdout,
            )?;
        }
        Ok(())
    }

    fn confirm_hook_execution(
        &self,
        template_root: &Path,
        skip: bool,
        hook_filename: &str,
    ) -> Result<bool> {
        let hook_file = template_root.join("hooks").join(hook_filename);
        if !hook_file.exists() {
            return Ok(false);
        }
        crate::prompt::confirm(
            skip,
            format!(
                "Allow hook '{hook_filename}' from '{}' to execute?",
                template_root.display()
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// Standalone helpers
// ---------------------------------------------------------------------------

fn load_and_validate_config(template_root: &PathBuf) -> Result<ConfigV1> {
    let config = Config::load_config(template_root)?;
    let Config::V1(config) = config;
    config.validate()?;
    Ok(config)
}

fn render_hook_runner(
    engine: &dyn TemplateRenderer,
    runner_tokens: &[String],
    answers: Option<&serde_json::Value>,
) -> Result<Vec<String>> {
    let empty_answers = serde_json::Value::Object(Default::default());
    let answers_ref = answers.unwrap_or(&empty_answers);
    runner_tokens
        .iter()
        .map(|token| engine.render(token, answers_ref, Some("hook_runner")))
        .collect()
}

/// Add import templates from `template_root` to the engine (mirrors Runner::add_templates_in_renderer).
fn add_templates_in_renderer(
    template_root: &Path,
    config: &ConfigV1,
    engine: &mut dyn TemplateRenderer,
) {
    let import_root = if let Some(ref s) = config.import_root {
        let p = Path::new(s);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            template_root.join(p)
        }
    } else {
        template_root.to_path_buf()
    };

    if config.template_globs.is_empty() {
        return;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in &config.template_globs {
        let full = import_root.join(pattern);
        if let Ok(g) = Glob::new(&full.to_string_lossy()) {
            builder.add(g);
        }
    }
    let globset = match builder.build() {
        Ok(gs) => gs,
        Err(_) => return,
    };

    WalkDir::new(&import_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && globset.is_match(e.path()))
        .filter_map(|e| {
            let path = e.path();
            let rel = path.strip_prefix(&import_root).ok()?;
            let name = rel.to_str()?.to_owned();
            std::fs::read_to_string(path).ok().map(|c| (name, c))
        })
        .for_each(|(name, content)| {
            if let Err(e) = engine.add_template(&name, &content) {
                log::warn!("Failed to add template {name}: {e}");
            }
        });
}

/// Clone a git repository into a sub-directory of `parent` and return its `LoadedTemplate`.
fn clone_git_into_tmp(url: &str, parent: &Path) -> Result<crate::loader::LoadedTemplate> {
    use crate::loader::git::GitLoader;
    use crate::loader::interface::TemplateLoader;

    // GitLoader clones into a relative path (repo name) from the *current* dir,
    // so we temporarily change into `parent`.
    let original_dir = std::env::current_dir()?;
    std::fs::create_dir_all(parent)?;
    std::env::set_current_dir(parent)?;

    let result = GitLoader::new(url.to_string(), true).load();

    // Always restore the original directory
    let _ = std::env::set_current_dir(&original_dir);

    result
}
