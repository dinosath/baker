use crate::{
    cli::SkipConfirm,
    error::Result,
    ioutils::{copy_file, create_dir_all, write_file},
    prompt::confirm,
    template::{operation::TemplateOperation, processor::TemplateProcessor},
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Handles the processing of template files and directories
pub struct FileProcessor<'a> {
    processor: TemplateProcessor<'a, PathBuf>,
    skip_confirms: &'a [SkipConfirm],
}

impl<'a> FileProcessor<'a> {
    pub fn new(
        processor: TemplateProcessor<'a, PathBuf>,
        skip_confirms: &'a [SkipConfirm],
    ) -> Self {
        Self { processor, skip_confirms }
    }

    /// Processes all files in the template directory
    pub fn process_all_files(&self, template_root: &Path) -> Result<()> {
        for dir_entry in WalkDir::new(template_root) {
            let template_entry = dir_entry?.path().to_path_buf();
            match self.processor.process(template_entry) {
                Ok(file_operation) => {
                    let user_confirmed_overwrite =
                        self.handle_file_operation(&file_operation)?;
                    let message = file_operation.get_message(user_confirmed_overwrite);
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
        match file_operation {
            TemplateOperation::Write { target, target_exists, content, .. } => {
                let skip_prompt = self.should_skip_overwrite_prompt(*target_exists);
                let user_confirmed =
                    confirm(skip_prompt, format!("Overwrite {}?", target.display()))?;

                if user_confirmed {
                    write_file(content, target)?;
                }
                Ok(user_confirmed)
            }
            TemplateOperation::Copy { target, target_exists, source, .. } => {
                let skip_prompt = self.should_skip_overwrite_prompt(*target_exists);
                let user_confirmed =
                    confirm(skip_prompt, format!("Overwrite {}?", target.display()))?;

                if user_confirmed {
                    copy_file(source, target)?;
                }
                Ok(user_confirmed)
            }
            TemplateOperation::CreateDirectory { target, target_exists } => {
                if !target_exists {
                    create_dir_all(target)?;
                }
                Ok(true)
            }
            TemplateOperation::Ignore { .. } => Ok(true),
        }
    }

    /// Determines if overwrite prompts should be skipped
    fn should_skip_overwrite_prompt(&self, target_exists: bool) -> bool {
        self.skip_confirms.contains(&SkipConfirm::All)
            || self.skip_confirms.contains(&SkipConfirm::Overwrite)
            || !target_exists
    }
}
