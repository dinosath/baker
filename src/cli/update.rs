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
use tempfile::TempDir;
use walkdir::WalkDir;

// Public entry point

/// Main entry point for `baker update`.
pub fn run_update(args: UpdateArgs) -> Result<()> {
    UpdateRunner::new(args).run()
}

/// Update a generated project using an explicit working directory.
pub fn run_update_in_dir(args: UpdateArgs, working_dir: PathBuf) -> Result<()> {
    UpdateRunner::with_working_dir(args, working_dir).run()
}

struct UpdateRunner {
    args: UpdateArgs,
    working_dir: PathBuf,
}

impl UpdateRunner {
    fn new(args: UpdateArgs) -> Self {
        Self {
            args,
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    fn with_working_dir(args: UpdateArgs, working_dir: PathBuf) -> Self {
        Self { args, working_dir }
    }

    /// Runs the update workflow:
    /// determines metadata filename, reads saved metadata, re-fetches template,
    /// compares sources, merges answers, loads config, builds context and engine,
    /// runs hooks, collects answers, processes templates, and writes updated metadata.
    fn run(self) -> Result<()> {
        let file_name = self
            .args
            .generated_file
            .as_deref()
            .unwrap_or(crate::constants::DEFAULT_GENERATED_FILE_NAME);

        let cwd = self.working_dir.clone();
        let meta: BakerGenerated = generated::read(&cwd, file_name)?;

        log::info!("Found generated metadata (generated_at={})", meta.generated_at);

        let skip_overwrite = self.should_skip_overwrite_prompts();
        let (loaded, _tmp_guard) =
            self.fetch_updated_template(&meta.template, skip_overwrite)?;

        if self.sources_are_identical(&meta.template, &loaded.source) {
            println!("Template has not changed since last generation — nothing to do.");
            return Ok(());
        }

        let merged_answers = self.merge_answers(meta.answers.clone())?;

        let config = load_and_validate_config(&loaded.root)?;

        let conflict_style: Option<ConflictStyle> =
            self.args.conflict_style.or(config.conflict_marker_style);

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

        let mut engine = get_template_engine();
        add_templates_in_renderer(&loaded.root, context.config(), &mut engine);

        let execute_hooks = self.confirm_hooks(&context, &engine)?;

        let pre_hook_output =
            self.maybe_run_pre_hook(&context, &engine, execute_hooks)?;

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

        let bakerignore = parse_bakerignore_file(context.template_root())?;
        let processor = TemplateProcessor::new(&engine, &context, &bakerignore);
        let file_processor = FileProcessor::new(processor, &context);
        file_processor.process_all_files()?;

        self.maybe_run_post_hook(&context, &engine, execute_hooks)?;

        if context.dry_run() {
            log::info!(
                "[DRY RUN] Would write updated generated metadata to '{}'",
                cwd.join(file_name).display()
            );
        } else {
            let answers =
                generated::strip_secret_answers(context.answers(), context.config());
            let new_meta = BakerGenerated::new(loaded.source, answers);
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

    /// Re-fetches the template from its original source.
    ///
    /// For git sources, clones into a temp directory and returns both the loaded
    /// template and the `TempDir` guard (RAII cleanup on drop). For filesystem
    /// sources, loads directly.
    fn fetch_updated_template(
        &self,
        stored: &TemplateSourceInfo,
        skip_overwrite: bool,
    ) -> Result<(crate::loader::LoadedTemplate, Option<TempDir>)> {
        match stored {
            TemplateSourceInfo::Git { url, .. } => {
                let tmp = TempDir::new()?;
                let tmp_path = tmp.path().to_path_buf();
                let loaded = clone_git_into_tmp(url, &tmp_path)?;
                Ok((loaded, Some(tmp)))
            }
            TemplateSourceInfo::Filesystem { path, .. } => {
                let loaded = get_template(path.as_str(), skip_overwrite)?;
                Ok((loaded, None))
            }
        }
    }

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

    /// Merges saved answers with CLI overrides (--answers-file, then --answers).
    fn merge_answers(&self, saved: serde_json::Value) -> Result<serde_json::Value> {
        let mut base = match saved {
            serde_json::Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };

        if let Some(ref path) = self.args.answers_file {
            let content = std::fs::read_to_string(path)?;
            let parsed: serde_json::Value = serde_json::from_str(&content)?;
            match parsed {
                serde_json::Value::Object(extra) => base.extend(extra),
                _ => return Err(crate::error::Error::AnswersNotObject),
            }
        }

        if let Some(ref answers_str) = self.args.answers {
            let raw = if answers_str == crate::constants::STDIN_INDICATOR {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
                buf
            } else {
                answers_str.clone()
            };
            let parsed: serde_json::Value = serde_json::from_str(&raw)?;
            match parsed {
                serde_json::Value::Object(extra) => base.extend(extra),
                _ => return Err(crate::error::Error::AnswersNotObject),
            }
        }

        Ok(serde_json::Value::Object(base))
    }

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

    fn maybe_run_pre_hook(
        &self,
        context: &GenerationContext,
        engine: &dyn TemplateRenderer,
        execute_hooks: bool,
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

        if execute_hooks {
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
        execute_hooks: bool,
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

        if execute_hooks {
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

    /// Single combined prompt for both pre and post hooks (mirrors runner.rs behaviour).
    fn confirm_hooks(
        &self,
        context: &GenerationContext,
        engine: &dyn TemplateRenderer,
    ) -> Result<bool> {
        let config = context.config();
        let pre_hook_filename = engine
            .render(
                &config.pre_hook_filename,
                &json!({}),
                Some(&config.pre_hook_filename),
            )
            .unwrap_or_else(|_| config.pre_hook_filename.clone());
        let post_hook_filename = engine
            .render(
                &config.post_hook_filename,
                &json!({}),
                Some(&config.post_hook_filename),
            )
            .unwrap_or_else(|_| config.post_hook_filename.clone());

        let pre_hook_file =
            context.template_root().join("hooks").join(&pre_hook_filename);
        let post_hook_file =
            context.template_root().join("hooks").join(&post_hook_filename);

        if !pre_hook_file.exists() && !post_hook_file.exists() {
            return Ok(false);
        }

        if context.dry_run() {
            return Ok(false);
        }

        let mut hook_list = String::new();
        if pre_hook_file.exists() {
            hook_list.push_str(&format!("{}\n", pre_hook_file.display()));
        }
        if post_hook_file.exists() {
            hook_list.push_str(&format!("{}\n", post_hook_file.display()));
        }

        crate::prompt::confirm(
            self.should_skip_hook_prompts(),
            format!(
                "WARNING: This template contains the following hooks that will execute commands on your system:\n{hook_list}Do you want to run these hooks?",
            ),
        )
    }
}

// Standalone helpers

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

    std::fs::create_dir_all(parent)?;
    GitLoader::new(url.to_string(), true).load_into_parent(parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cli::SkipConfirm, config::Config, loader::TemplateSourceInfo,
        renderer::TemplateRenderer,
    };
    use serde_json::json;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    fn default_update_args() -> UpdateArgs {
        UpdateArgs {
            generated_file: None,
            answers: None,
            answers_file: None,
            conflict_style: None,
            dry_run: false,
            skip_confirms: vec![],
            non_interactive: false,
        }
    }

    fn parse_config(raw: &str) -> ConfigV1 {
        let config: Config = serde_yaml::from_str(raw).expect("valid config yaml");
        let Config::V1(v1) = config;
        v1
    }

    fn minimal_config() -> ConfigV1 {
        parse_config(
            r#"
schemaVersion: v1
questions: {}
"#,
        )
    }

    fn init_git_repo(path: &Path) -> String {
        let repo = git2::Repository::init(path).expect("init repository");
        fs::write(path.join("README.md"), "hello").expect("write file");

        let mut index = repo.index().expect("open index");
        index.add_path(Path::new("README.md")).expect("add file to index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = git2::Signature::now("tester", "tester@example.com")
            .expect("create signature");

        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("create commit")
            .to_string()
    }

    #[test]
    fn sources_are_identical_checks_git_and_filesystem_variants() {
        let runner = UpdateRunner::new(default_update_args());

        let git_a = TemplateSourceInfo::Git {
            url: "https://example.com/repo.git".to_string(),
            commit: "abc".to_string(),
            tag: None,
        };
        let git_b = TemplateSourceInfo::Git {
            url: "https://example.com/repo.git".to_string(),
            commit: "abc".to_string(),
            tag: Some("v1.0.0".to_string()),
        };
        let git_c = TemplateSourceInfo::Git {
            url: "https://example.com/repo.git".to_string(),
            commit: "def".to_string(),
            tag: None,
        };

        assert!(runner.sources_are_identical(&git_a, &git_b));
        assert!(!runner.sources_are_identical(&git_a, &git_c));

        let fs_a = TemplateSourceInfo::Filesystem {
            path: "/tmp/template".to_string(),
            hash: "111".to_string(),
        };
        let fs_b = TemplateSourceInfo::Filesystem {
            path: "/tmp/template".to_string(),
            hash: "111".to_string(),
        };
        let fs_c = TemplateSourceInfo::Filesystem {
            path: "/tmp/template".to_string(),
            hash: "222".to_string(),
        };

        assert!(runner.sources_are_identical(&fs_a, &fs_b));
        assert!(!runner.sources_are_identical(&fs_a, &fs_c));
        assert!(!runner.sources_are_identical(&git_a, &fs_a));
    }

    #[test]
    fn merge_answers_merges_saved_file_and_inline_json() {
        let dir = tempdir().expect("create temp dir");
        let answers_file = dir.path().join("answers.json");
        fs::write(&answers_file, r#"{"from_file":2}"#).expect("write answers file");

        let mut args = default_update_args();
        args.answers_file = Some(answers_file);
        args.answers = Some(r#"{"inline":3}"#.to_string());

        let runner = UpdateRunner::new(args);
        let merged = runner.merge_answers(json!({"saved": 1})).expect("merge answers");

        assert_eq!(merged["saved"], json!(1));
        assert_eq!(merged["from_file"], json!(2));
        assert_eq!(merged["inline"], json!(3));
    }

    #[test]
    fn merge_answers_errors_when_non_object_override_is_used() {
        let mut args = default_update_args();
        args.answers = Some("[]".to_string());

        let runner = UpdateRunner::new(args);
        let err = runner.merge_answers(json!({})).expect_err("expected error");

        assert!(matches!(err, crate::error::Error::AnswersNotObject));
    }

    #[test]
    fn merge_answers_uses_empty_object_when_saved_answers_are_not_object() {
        let mut args = default_update_args();
        args.answers = Some(r#"{"k":"v"}"#.to_string());
        let runner = UpdateRunner::new(args);

        let merged = runner.merge_answers(json!("old")).expect("merge should work");
        assert_eq!(merged, json!({"k": "v"}));
    }

    #[test]
    fn skip_prompt_flags_work_for_overwrite_and_hooks() {
        let mut args = default_update_args();
        args.skip_confirms = vec![SkipConfirm::Overwrite];
        let runner = UpdateRunner::new(args);
        assert!(runner.should_skip_overwrite_prompts());
        assert!(!runner.should_skip_hook_prompts());

        let mut args = default_update_args();
        args.skip_confirms = vec![SkipConfirm::Hooks];
        let runner = UpdateRunner::new(args);
        assert!(!runner.should_skip_overwrite_prompts());
        assert!(runner.should_skip_hook_prompts());

        let mut args = default_update_args();
        args.skip_confirms = vec![SkipConfirm::All];
        let runner = UpdateRunner::new(args);
        assert!(runner.should_skip_overwrite_prompts());
        assert!(runner.should_skip_hook_prompts());
    }

    #[test]
    fn load_and_validate_config_handles_valid_and_invalid_configs() {
        let dir = tempdir().expect("create temp dir");

        fs::write(dir.path().join("baker.yaml"), "schemaVersion: v1\nquestions: {}\n")
            .expect("write config");

        let valid = load_and_validate_config(&dir.path().to_path_buf())
            .expect("valid config should load");
        assert_eq!(valid.template_suffix, ".baker.j2");

        fs::write(
            dir.path().join("baker.yaml"),
            "schemaVersion: v1\ntemplate_suffix: invalid\nquestions: {}\n",
        )
        .expect("write invalid config");

        let err = load_and_validate_config(&dir.path().to_path_buf())
            .expect_err("invalid config should fail");
        assert!(matches!(err, crate::error::Error::ConfigValidation(_)));
    }

    #[test]
    fn render_hook_runner_renders_tokens_with_and_without_answers() {
        let engine = crate::template::get_template_engine();

        let rendered = render_hook_runner(
            &engine,
            &["echo".to_string(), "{{ name }}".to_string()],
            Some(&json!({"name": "baker"})),
        )
        .expect("render hook runner with answers");

        assert_eq!(rendered, vec!["echo".to_string(), "baker".to_string()]);

        let rendered_without_answers =
            render_hook_runner(&engine, &["plain".to_string()], None)
                .expect("render hook runner without answers");
        assert_eq!(rendered_without_answers, vec!["plain".to_string()]);
    }

    #[test]
    fn add_templates_in_renderer_adds_templates_matching_globs() {
        let template_dir = tempdir().expect("create temp dir");
        let import_root = template_dir.path().join("imports");
        fs::create_dir_all(&import_root).expect("create imports dir");

        fs::write(import_root.join("hello.j2"), "Hello {{ name }}")
            .expect("write template file");
        fs::write(import_root.join("skip.txt"), "skip").expect("write non-template file");

        let config = parse_config(
            r#"
schemaVersion: v1
import_root: imports
template_globs: ["**/*.j2"]
questions: {}
"#,
        );

        let mut engine = crate::template::get_template_engine();
        add_templates_in_renderer(template_dir.path(), &config, &mut engine);

        let rendered = engine
            .render("{% include \"hello.j2\" %}", &json!({"name": "World"}), Some("test"))
            .expect("render include");
        assert_eq!(rendered, "Hello World");
    }

    #[test]
    fn maybe_run_hooks_handles_missing_files_dry_run_and_skip_execution() {
        let template_dir = tempdir().expect("template dir");
        let output_dir = tempdir().expect("output dir");
        let hooks_dir = template_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");
        fs::write(hooks_dir.join("pre.sh"), "#!/bin/sh\necho pre\n")
            .expect("write pre hook");
        fs::write(hooks_dir.join("post.sh"), "#!/bin/sh\necho post\n")
            .expect("write post hook");

        let config = parse_config(
            r#"
schemaVersion: v1
pre_hook_filename: pre.sh
post_hook_filename: post.sh
questions: {}
"#,
        );

        let mut dry_context = GenerationContext::new(
            template_dir.path().to_path_buf(),
            output_dir.path().to_path_buf(),
            config,
            vec![],
            true,
            false,
            None,
        );
        dry_context.set_answers(json!({"name": "baker"}));
        let runner = UpdateRunner::new(default_update_args());
        let engine = crate::template::get_template_engine();

        assert_eq!(
            runner
                .maybe_run_pre_hook(&dry_context, &engine, true)
                .expect("dry-run pre hook"),
            None
        );
        runner
            .maybe_run_post_hook(&dry_context, &engine, true)
            .expect("dry-run post hook");

        let config = parse_config(
            r#"
schemaVersion: v1
pre_hook_filename: pre.sh
post_hook_filename: post.sh
questions: {}
"#,
        );
        let mut normal_context = GenerationContext::new(
            template_dir.path().to_path_buf(),
            output_dir.path().to_path_buf(),
            config,
            vec![],
            false,
            false,
            None,
        );
        normal_context.set_answers(json!({"name": "baker"}));

        assert_eq!(
            runner
                .maybe_run_pre_hook(&normal_context, &engine, false)
                .expect("skip pre hook execution"),
            None
        );
        runner
            .maybe_run_post_hook(&normal_context, &engine, false)
            .expect("skip post hook execution");
    }

    #[test]
    fn confirm_hooks_respects_missing_hooks_dry_run_and_skip_flag() {
        let template_dir = tempdir().expect("template dir");
        let output_dir = tempdir().expect("output dir");
        let hooks_dir = template_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");

        let engine = crate::template::get_template_engine();

        // No hooks => false
        let context_no_hooks = GenerationContext::new(
            template_dir.path().to_path_buf(),
            output_dir.path().to_path_buf(),
            minimal_config(),
            vec![],
            false,
            false,
            None,
        );
        let runner = UpdateRunner::new(default_update_args());
        assert!(!runner
            .confirm_hooks(&context_no_hooks, &engine)
            .expect("confirm hooks with no files"));

        // Hooks present but dry-run => false
        fs::write(hooks_dir.join("pre"), "#!/bin/sh\n").expect("write pre");
        let context_dry_run = GenerationContext::new(
            template_dir.path().to_path_buf(),
            output_dir.path().to_path_buf(),
            minimal_config(),
            vec![],
            true,
            false,
            None,
        );
        assert!(!runner
            .confirm_hooks(&context_dry_run, &engine)
            .expect("confirm hooks in dry run"));

        // Hooks present and skip flag set => true (no interactive prompt)
        fs::write(hooks_dir.join("post"), "#!/bin/sh\n").expect("write post");
        let mut args = default_update_args();
        args.skip_confirms = vec![SkipConfirm::Hooks];
        let runner = UpdateRunner::new(args);
        let context = GenerationContext::new(
            template_dir.path().to_path_buf(),
            output_dir.path().to_path_buf(),
            minimal_config(),
            vec![],
            false,
            false,
            None,
        );
        assert!(runner
            .confirm_hooks(&context, &engine)
            .expect("confirm hooks with skip flag"));
    }

    #[test]
    fn clone_git_into_tmp_clones_repo_under_parent_directory() {
        let source_parent = tempdir().expect("source parent");
        let source_repo = source_parent.path().join("source_repo");
        fs::create_dir_all(&source_repo).expect("create source repo dir");
        let _commit = init_git_repo(&source_repo);

        let parent = tempdir().expect("parent dir");

        let loaded = clone_git_into_tmp(
            source_repo.to_str().expect("source repo path"),
            parent.path(),
        )
        .expect("clone into temp");

        assert!(loaded.root.starts_with(parent.path()));
        assert!(loaded.root.exists(), "cloned path should exist");
    }

    #[test]
    fn fetch_updated_template_loads_filesystem_source() {
        let template_dir = tempdir().expect("template dir");
        fs::write(template_dir.path().join("README.md"), "content")
            .expect("write template file");

        let source = TemplateSourceInfo::Filesystem {
            path: template_dir.path().to_string_lossy().to_string(),
            hash: String::new(),
        };

        let runner = UpdateRunner::new(default_update_args());
        let (loaded, temp_guard) = runner
            .fetch_updated_template(&source, true)
            .expect("load filesystem template");

        assert!(temp_guard.is_none());
        assert_eq!(loaded.root, template_dir.path().to_path_buf());
    }
}
