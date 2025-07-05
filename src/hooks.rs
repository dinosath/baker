use serde::Serialize;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::ioutils::path_to_str;

/// Structure representing data passed to hook scripts.
///
/// This data is serialized to JSON and passed to hook scripts via stdin.
#[derive(Serialize)]
struct Output<'a> {
    /// Absolute path to the template directory
    pub template_dir: &'a str,
    /// Absolute path to the output directory
    pub output_dir: &'a str,
    /// Context data for template rendering
    pub answers: Option<&'a serde_json::Value>,
}

/// Executes a hook script with the provided context.
///
/// # Arguments
/// * `template_dir` - Path to the template directory
/// * `output_dir` - Path to the output directory
/// * `script_path` - Path to the hook script to execute
/// * `context` - Template context data
///
/// # Returns
/// * `Result<Option<String>>` - Success or error status of hook execution, with stdout content
///
/// # Notes
/// - Hook scripts receive context data as JSON via stdin
/// - Hooks must be executable files
/// - Non-zero exit codes from hooks are treated as errors
pub fn run_hook<P: AsRef<Path>>(
    template_dir: P,
    output_dir: P,
    hook_path: P,
    answers: Option<&serde_json::Value>,
) -> Result<Option<String>> {
    let hook_path = hook_path.as_ref();

    let template_dir = path_to_str(&template_dir)?;
    let output_dir = path_to_str(&output_dir)?;

    let output = Output { template_dir, output_dir, answers };

    // Properly handle serialization errors
    let output_data = serde_json::to_vec(&output).map_err(Error::JSONParseError)?;

    if !hook_path.exists() {
        return Ok(None);
    }

    let mut child = Command::new(hook_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Write context to stdin and close it
    if let Some(mut stdin) = child.stdin.take() {
        // Helper closure to handle broken pipe errors consistently
        let handle_write_error = |e: std::io::Error, operation: &str| {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                log::debug!("Hook closed stdin before {operation} (broken pipe) - this is normal for hooks that don't read input");
            } else {
                log::warn!("Failed to {operation}: {e}");
            }
        };

        // Write context data to stdin
        if let Err(e) = stdin.write_all(&output_data) {
            handle_write_error(e, "write context data to hook stdin");
        } else if let Err(e) = stdin.write_all(b"\n") {
            handle_write_error(e, "write newline to hook stdin");
        }

        // Explicitly close stdin to signal end of input
        drop(stdin);
    }

    // Read stdout before waiting for the process to complete
    let stdout_output = match child.stdout.take() {
        Some(stdout) => {
            let mut output = String::new();
            let mut reader = BufReader::new(stdout);
            reader.read_to_string(&mut output)?;
            Some(output)
        }
        None => None,
    };

    // Wait for the process to complete
    let status = child.wait()?;

    if !status.success() {
        return Err(Error::HookExecutionError {
            script: hook_path.display().to_string(),
            status,
        });
    }

    Ok(stdout_output)
}
