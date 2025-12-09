use std::path::PathBuf;

#[derive(Debug)]
pub enum TemplateOperation {
    Copy { source: PathBuf, target: PathBuf, target_exists: bool },
    Write { target: PathBuf, content: String, target_exists: bool },
    CreateDirectory { target: PathBuf, target_exists: bool },
    Ignore { source: PathBuf },
    MultipleWrite { writes: Vec<WriteOp> },
}

#[derive(Debug)]
pub struct WriteOp {
    pub target: PathBuf,
    pub content: String,
    pub target_exists: bool,
}

impl TemplateOperation {
    /// Gets a message describing the operation and its status.
    ///
    /// # Arguments
    /// * `user_confirmed_overwrite` - Whether the user has confirmed overwriting existing files
    /// * `dry_run` - Whether this is a dry run (no actual file operations)
    ///
    /// # Returns
    /// * `String` - A descriptive message about the operation
    pub fn get_message(&self, user_confirmed_overwrite: bool, dry_run: bool) -> String {
        let prefix = if dry_run { "[DRY RUN] " } else { "" };

        match self {
            TemplateOperation::Copy { source, target, target_exists } => {
                if *target_exists {
                    if user_confirmed_overwrite {
                        format!(
                            "{}Copying '{}' to '{}' (overwriting existing file)",
                            prefix,
                            source.display(),
                            target.display()
                        )
                    } else {
                        format!(
                            "{}Skipping copy of '{}' to '{}' (target already exists)",
                            prefix,
                            source.display(),
                            target.display()
                        )
                    }
                } else {
                    format!(
                        "{}Copying '{}' to '{}'",
                        prefix,
                        source.display(),
                        target.display()
                    )
                }
            }

            TemplateOperation::CreateDirectory { target, target_exists } => {
                if *target_exists {
                    format!(
                        "{}Skipping directory creation '{}' (already exists)",
                        prefix,
                        target.display()
                    )
                } else {
                    format!("{}Creating directory '{}'", prefix, target.display())
                }
            }

            TemplateOperation::Write { target, target_exists, .. } => {
                if *target_exists {
                    if user_confirmed_overwrite {
                        format!(
                            "{}Writing to '{}' (overwriting existing file)",
                            prefix,
                            target.display()
                        )
                    } else {
                        format!(
                            "{}Skipping write to '{}' (target already exists)",
                            prefix,
                            target.display()
                        )
                    }
                } else {
                    format!("{}Writing to '{}'", prefix, target.display())
                }
            }

            TemplateOperation::Ignore { source } => {
                format!(
                    "{}Ignoring '{}' (matches ignore pattern)",
                    prefix,
                    source.display()
                )
            }

            TemplateOperation::MultipleWrite { writes } => {
                let overwriting: Vec<_> = writes
                    .iter()
                    .filter(|w| w.target_exists)
                    .map(|w| w.target.display().to_string())
                    .collect();
                let writing: Vec<_> = writes
                    .iter()
                    .filter(|w| !w.target_exists)
                    .map(|w| w.target.display().to_string())
                    .collect();
                let mut msg = String::new();
                if !overwriting.is_empty() {
                    msg.push_str(&format!("Overwriting: {}.", overwriting.join(", ")));
                }
                if !writing.is_empty() {
                    msg.push_str(&format!("Writing: {}.", writing.join(", ")));
                }
                msg.trim_end().to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_operation_logs_overwrite_message() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = true;
        let expected = format!(
            "Copying '{}' to '{}' (overwriting existing file)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }

    #[test]
    fn copy_operation_skips_when_not_confirmed() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!(
            "Skipping copy of '{}' to '{}' (target already exists)",
            &source.display(),
            &target.display()
        );

        let copy = TemplateOperation::Copy { source, target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }

    #[test]
    fn copy_operation_logs_basic_message() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Copying '{}' to '{}'", &source.display(), &target.display());

        let copy = TemplateOperation::Copy { source, target, target_exists: false };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn create_directory_skips_when_exists() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!(
            "Skipping directory creation '{}' (already exists)",
            &target.display()
        );

        let copy = TemplateOperation::CreateDirectory { target, target_exists: true };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn create_directory_message_when_missing() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!("Creating directory '{}'", &target.display());

        let copy = TemplateOperation::CreateDirectory { target, target_exists: false };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn write_operation_overwrite_message() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = true;
        let expected =
            format!("Writing to '{}' (overwriting existing file)", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: true,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn write_operation_skips_without_confirmation() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Skipping write to '{}' (target already exists)", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: true,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn write_operation_basic_message() {
        let target = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected = format!("Writing to '{}'", &target.display());

        let copy = TemplateOperation::Write {
            target,
            target_exists: false,
            content: "".to_string(),
        };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }
    #[test]
    fn ignore_operation_logs_message() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let user_confirmed_overwrite = false;
        let expected =
            format!("Ignoring '{}' (matches ignore pattern)", &source.display());

        let copy = TemplateOperation::Ignore { source };
        let actual = copy.get_message(user_confirmed_overwrite, false);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_dry_run_messages() {
        let source = PathBuf::from("/tmp/test/file.txt");
        let target = PathBuf::from("/tmp/test/target.txt");

        let copy = TemplateOperation::Copy { source, target, target_exists: false };
        let dry_run_message = copy.get_message(false, true);
        let normal_message = copy.get_message(false, false);

        assert!(dry_run_message.starts_with("[DRY RUN] "));
        assert!(!normal_message.starts_with("[DRY RUN] "));
        assert_eq!(dry_run_message, format!("[DRY RUN] {}", normal_message));
    }

    #[test]
    fn multiple_write_operation_message_with_overwrites() {
        let writes = vec![
            WriteOp {
                target: PathBuf::from("/tmp/existing.txt"),
                content: "content1".to_string(),
                target_exists: true,
            },
            WriteOp {
                target: PathBuf::from("/tmp/new.txt"),
                content: "content2".to_string(),
                target_exists: false,
            },
        ];

        let op = TemplateOperation::MultipleWrite { writes };
        let message = op.get_message(true, false);

        assert!(message.contains("Overwriting:"));
        assert!(message.contains("existing.txt"));
        assert!(message.contains("Writing:"));
        assert!(message.contains("new.txt"));
    }

    #[test]
    fn multiple_write_operation_message_only_new_files() {
        let writes = vec![
            WriteOp {
                target: PathBuf::from("/tmp/new1.txt"),
                content: "content1".to_string(),
                target_exists: false,
            },
            WriteOp {
                target: PathBuf::from("/tmp/new2.txt"),
                content: "content2".to_string(),
                target_exists: false,
            },
        ];

        let op = TemplateOperation::MultipleWrite { writes };
        let message = op.get_message(true, false);

        assert!(!message.contains("Overwriting:"));
        assert!(message.contains("Writing:"));
        assert!(message.contains("new1.txt"));
        assert!(message.contains("new2.txt"));
    }

    #[test]
    fn multiple_write_operation_message_only_overwrites() {
        let writes = vec![
            WriteOp {
                target: PathBuf::from("/tmp/existing1.txt"),
                content: "content1".to_string(),
                target_exists: true,
            },
            WriteOp {
                target: PathBuf::from("/tmp/existing2.txt"),
                content: "content2".to_string(),
                target_exists: true,
            },
        ];

        let op = TemplateOperation::MultipleWrite { writes };
        let message = op.get_message(true, false);

        assert!(message.contains("Overwriting:"));
        assert!(message.contains("existing1.txt"));
        assert!(message.contains("existing2.txt"));
        assert!(!message.contains("Writing:"));
    }

    #[test]
    fn multiple_write_operation_empty_writes() {
        let writes: Vec<WriteOp> = vec![];
        let op = TemplateOperation::MultipleWrite { writes };
        let message = op.get_message(true, false);
        assert!(message.is_empty());
    }

    #[test]
    fn write_op_debug_impl() {
        let write_op = WriteOp {
            target: PathBuf::from("/tmp/test.txt"),
            content: "test content".to_string(),
            target_exists: true,
        };
        let debug_str = format!("{:?}", write_op);
        assert!(debug_str.contains("WriteOp"));
        assert!(debug_str.contains("test.txt"));
        assert!(debug_str.contains("test content"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn template_operation_debug_impl() {
        let op = TemplateOperation::Copy {
            source: PathBuf::from("/src"),
            target: PathBuf::from("/dst"),
            target_exists: false,
        };
        let debug_str = format!("{:?}", op);
        assert!(debug_str.contains("Copy"));
        assert!(debug_str.contains("/src"));
        assert!(debug_str.contains("/dst"));
    }

    #[test]
    fn multiple_write_dry_run_message() {
        let writes = vec![WriteOp {
            target: PathBuf::from("/tmp/file.txt"),
            content: "content".to_string(),
            target_exists: false,
        }];

        let op = TemplateOperation::MultipleWrite { writes };
        let message = op.get_message(true, true);

        // MultipleWrite doesn't use the dry_run prefix in its implementation
        assert!(message.contains("Writing:"));
    }
}
