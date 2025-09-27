use crate::{
    cli::{context::GenerationContext, SkipConfirm},
    error::{Error, Result},
    prompt::confirm,
    template::{
        operation::{TemplateOperation, WriteOp},
        processor::TemplateProcessor,
    },
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Handles the processing of template files and directories
pub struct FileProcessor<'a> {
    processor: TemplateProcessor<'a, PathBuf>,
    context: &'a GenerationContext,
}

impl<'a> FileProcessor<'a> {
    pub fn new(
        processor: TemplateProcessor<'a, PathBuf>,
        context: &'a GenerationContext,
    ) -> Self {
        Self { processor, context }
    }

    /// Processes all files in the template directory
    pub fn process_all_files(&self) -> Result<()> {
        for dir_entry in WalkDir::new(self.context.template_root()) {
            let template_entry = dir_entry?.path().to_path_buf();
            match self.processor.process(template_entry) {
                Ok(file_operation) => {
                    let user_confirmed_overwrite = match &file_operation {
                        TemplateOperation::Ignore { .. } => continue,
                        _ => match self.handle_file_operation(&file_operation) {
                            Ok(confirmed) => confirmed,
                            Err(e) => {
                                log::error!("Failed to handle file operation: {e}");
                                continue;
                            }
                        },
                    };
                    let message = file_operation
                        .get_message(user_confirmed_overwrite, self.context.dry_run());
                    log::info!("{message}");
                }
                Err(e) => match e {
                    crate::error::Error::ProcessError { .. } => log::warn!("{e}"),
                    _ => log::error!("{e}"),
                },
            }
        }
        Ok(())
    }

    /// Handles a single file operation (write, copy, create directory, or ignore)
    fn handle_file_operation(&self, file_operation: &TemplateOperation) -> Result<bool> {
        log::debug!("Handling file operation: {file_operation:?}");
        match file_operation {
            TemplateOperation::Write { target, target_exists, content, .. } => {
                self.handle_write(target, *target_exists, content)
            }
            TemplateOperation::Copy { target, target_exists, source, .. } => {
                self.handle_copy(source, target, *target_exists)
            }
            TemplateOperation::CreateDirectory { target, target_exists } => {
                self.handle_create_dir(target, *target_exists)
            }
            TemplateOperation::Ignore { .. } => Ok(true),
            TemplateOperation::MultipleWrite { writes, .. } => {
                self.handle_multiple_write(writes)
            }
        }
    }

    fn handle_write(
        &self,
        target: &Path,
        target_exists: bool,
        content: &str,
    ) -> Result<bool> {
        let user_confirmed = self.confirm_overwrite(target, target_exists)?;
        if user_confirmed {
            self.write_file(content, target)?;
        }
        Ok(user_confirmed)
    }

    fn handle_copy(
        &self,
        source: &Path,
        target: &Path,
        target_exists: bool,
    ) -> Result<bool> {
        let user_confirmed = self.confirm_overwrite(target, target_exists)?;
        if user_confirmed {
            self.copy_file(source, target)?;
        }
        Ok(user_confirmed)
    }

    fn handle_create_dir(&self, target: &Path, target_exists: bool) -> Result<bool> {
        if !target_exists {
            self.create_dir_all(target)?;
        }
        Ok(true)
    }

    fn handle_multiple_write(&self, writes: &[WriteOp]) -> Result<bool> {
        for write in writes {
            let user_confirmed =
                self.confirm_overwrite(&write.target, write.target_exists)?;
            if user_confirmed {
                self.write_file(&write.content, &write.target)?;
            }
        }
        Ok(true)
    }

    fn confirm_overwrite(&self, target: &Path, target_exists: bool) -> Result<bool> {
        let skip_prompt = self.should_skip_overwrite_prompt(target_exists);
        confirm(skip_prompt, format!("Overwrite {}?", target.display()))
    }

    /// Copy a file from source to destination, creating parent directories if needed.
    fn copy_file<P: AsRef<Path>>(&self, source_path: P, dest_path: P) -> Result<()> {
        let dest_path = dest_path.as_ref();

        if self.context.dry_run() {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            self.create_dir_all(parent)?;
        }

        Ok(std::fs::copy(source_path.as_ref(), dest_path).map(|_| ())?)
    }

    /// Write content to a file, creating parent directories if needed.
    fn write_file<P: AsRef<Path>>(&self, content: &str, dest_path: P) -> Result<()> {
        let dest_path = dest_path.as_ref();

        if self.context.dry_run() {
            return Ok(());
        }

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            self.create_dir_all(parent)?;
        }

        std::fs::write(dest_path, content).map_err(Error::from)
    }

    /// Create directory and all parent directories if they don't exist.
    fn create_dir_all<P: AsRef<Path>>(&self, dest_path: P) -> Result<()> {
        if self.context.dry_run() {
            return Ok(());
        }

        std::fs::create_dir_all(dest_path.as_ref()).map_err(Error::from)
    }

    /// Determines if overwrite prompts should be skipped
    fn should_skip_overwrite_prompt(&self, target_exists: bool) -> bool {
        self.context.skip_confirms().contains(&SkipConfirm::All)
            || self.context.skip_confirms().contains(&SkipConfirm::Overwrite)
            || !target_exists
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::MiniJinjaRenderer;
    use globset::GlobSetBuilder;
    use indexmap::IndexMap;
    use serde_json::json;
    use tempfile::TempDir;

    fn build_file_processor(
        skip_confirms: Vec<SkipConfirm>,
    ) -> (TempDir, TempDir, FileProcessor<'static>) {
        let template_root = TempDir::new().unwrap();
        let output_root = TempDir::new().unwrap();
        let engine = Box::leak(Box::new(MiniJinjaRenderer::new()));
        let bakerignore = Box::leak(Box::new(GlobSetBuilder::new().build().unwrap()));

        let mut context = GenerationContext::new(
            template_root.path().to_path_buf(),
            output_root.path().to_path_buf(),
            crate::config::ConfigV1 {
                template_suffix: ".baker.j2".into(),
                loop_separator: "".into(),
                loop_content_separator: "".into(),
                template_globs: Vec::new(),
                questions: IndexMap::new(),
                post_hook_filename: "post".into(),
                pre_hook_filename: "pre".into(),
            },
            skip_confirms,
            false,
        );
        context.set_answers(json!({}));
        let context = Box::leak(Box::new(context));
        let processor = TemplateProcessor::new(&*engine, context, &*bakerignore);

        (template_root, output_root, FileProcessor::new(processor, context))
    }

    #[test]
    fn skips_overwrite_prompt_for_new_files() {
        let (_template_root, _output_root, processor) = build_file_processor(Vec::new());
        assert!(processor.should_skip_overwrite_prompt(false));
        assert!(!processor.should_skip_overwrite_prompt(true));
    }

    #[test]
    fn skips_overwrite_prompt_when_flagged() {
        let (_template_root, _output_root, processor) =
            build_file_processor(vec![SkipConfirm::Overwrite]);
        assert!(processor.should_skip_overwrite_prompt(true));
    }

    #[test]
    fn confirm_overwrite_short_circuits_when_skip_applies() {
        let (_template_root, output_root, processor) = build_file_processor(Vec::new());
        let target = output_root.path().join("new-file.txt");
        let result = processor.confirm_overwrite(&target, false).unwrap();
        assert!(result);
    }
}
