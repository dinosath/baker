use serde::Serialize;
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
}
