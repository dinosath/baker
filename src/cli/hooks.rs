use serde::Serialize;
use std::borrow::Cow;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

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
    runner: &[String],
) -> Result<Option<String>> {
    let hook_path = hook_path.as_ref();

    let template_dir = template_dir.as_ref().display().to_string();
    let output_dir = output_dir.as_ref().display().to_string();

    let output = Output { template_dir: &template_dir, output_dir: &output_dir, answers };

    // Properly handle serialization errors
    let output_data = serde_json::to_vec(&output).map_err(Error::JSONParseError)?;

    if !hook_path.exists() {
        return Ok(None);
    }

    log::debug!("Running hook {} via runner {:?}", hook_path.display(), runner);

    let mut command = if runner.is_empty() {
        Command::new(hook_path)
    } else {
        let mut cmd = Command::new(&runner[0]);
        if runner.len() > 1 {
            cmd.args(&runner[1..]);
        }
        cmd.arg(hook_path);
        cmd
    };

    let mut child = command
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
            let mut reader = BufReader::new(stdout);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            let decoded = String::from_utf8_lossy(&buffer);
            if matches!(decoded, Cow::Owned(_)) {
                log::warn!(
                    "Hook {} emitted non-UTF8 stdout; performing lossy conversion",
                    hook_path.display()
                );
            }
            Some(decoded.into_owned())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    #[test]
    fn executes_script_via_runner_on_unix() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("hook.sh");
        File::create(&script_path).unwrap().write_all(b"echo unix_runner").unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o644)).unwrap();

        let output = run_hook(
            temp_dir.path(),
            temp_dir.path(),
            &script_path,
            None,
            &["sh".to_string()],
        )
        .expect("hook execution")
        .expect("stdout");

        assert!(output.contains("unix_runner"));
    }

    #[cfg(windows)]
    #[test]
    fn executes_script_via_powershell_runner_on_windows() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("hook.ps1");
        File::create(&script_path)
            .unwrap()
            .write_all(b"Write-Output 'windows_runner'")
            .unwrap();

        let runner = vec![
            "powershell".to_string(),
            "-NoLogo".to_string(),
            "-NonInteractive".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-File".to_string(),
        ];

        let output =
            run_hook(temp_dir.path(), temp_dir.path(), &script_path, None, &runner)
                .expect("hook execution")
                .expect("stdout");

        assert!(output.contains("windows_runner"));
    }

    #[test]
    fn run_hook_returns_none_when_script_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent_hook.sh");

        let result =
            run_hook(temp_dir.path(), temp_dir.path(), &nonexistent_path, None, &[]);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_with_answers_json() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("hook.sh");
        // Script reads from stdin and outputs it
        File::create(&script_path).unwrap().write_all(b"#!/bin/sh\ncat").unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

        let answers = serde_json::json!({"name": "test_value"});
        let result =
            run_hook(temp_dir.path(), temp_dir.path(), &script_path, Some(&answers), &[]);

        assert!(result.is_ok());
        let output = result.unwrap().unwrap();
        assert!(output.contains("test_value"));
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_with_multiple_runner_args() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("hook.sh");
        File::create(&script_path).unwrap().write_all(b"echo 'multi_arg_test'").unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o644)).unwrap();

        // Note: This tests the runner with arguments
        let output = run_hook(
            temp_dir.path(),
            temp_dir.path(),
            &script_path,
            None,
            &["sh".to_string()],
        )
        .expect("hook execution")
        .expect("stdout");

        assert!(output.contains("multi_arg_test"));
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_failure_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("failing_hook.sh");
        File::create(&script_path).unwrap().write_all(b"#!/bin/sh\nexit 1").unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

        let result = run_hook(temp_dir.path(), temp_dir.path(), &script_path, None, &[]);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::HookExecutionError { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_that_ignores_stdin() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("ignore_stdin.sh");
        // Script ignores stdin completely
        File::create(&script_path)
            .unwrap()
            .write_all(b"#!/bin/sh\necho 'ignored stdin'")
            .unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

        let answers = serde_json::json!({"key": "value"});
        let result =
            run_hook(temp_dir.path(), temp_dir.path(), &script_path, Some(&answers), &[]);

        assert!(result.is_ok());
        let output = result.unwrap().unwrap();
        assert!(output.contains("ignored stdin"));
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_with_empty_runner() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("executable_hook.sh");
        File::create(&script_path)
            .unwrap()
            .write_all(b"#!/bin/sh\necho 'direct_execution'")
            .unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

        let result = run_hook(
            temp_dir.path(),
            temp_dir.path(),
            &script_path,
            None,
            &[], // Empty runner means direct execution
        );

        assert!(result.is_ok());
        let output = result.unwrap().unwrap();
        assert!(output.contains("direct_execution"));
    }

    #[test]
    fn test_output_struct_serialization() {
        let output = Output {
            template_dir: "/path/to/template",
            output_dir: "/path/to/output",
            answers: Some(&serde_json::json!({"key": "value"})),
        };

        let serialized = serde_json::to_string(&output).unwrap();
        assert!(serialized.contains("/path/to/template"));
        assert!(serialized.contains("/path/to/output"));
        assert!(serialized.contains("key"));
        assert!(serialized.contains("value"));
    }

    #[test]
    fn test_output_struct_serialization_without_answers() {
        let output =
            Output { template_dir: "/template", output_dir: "/output", answers: None };

        let serialized = serde_json::to_string(&output).unwrap();
        assert!(serialized.contains("/template"));
        assert!(serialized.contains("/output"));
        assert!(serialized.contains("null"));
    }
}
