//! Integration tests for `baker update`.
//!
//! These tests exercise the full update flow end-to-end using local filesystem
//! templates and (optionally) git-based templates.
//!
//! The git-based tests require Docker + Gitea and are therefore `#[ignore]`d
//! so they don't run in a standard `cargo test` invocation.
//! Run them explicitly with:
//!   `cargo test --test update_integration_tests -- --ignored`

mod utils;

use baker::cli::{run, run_update, GenerateArgs, SkipConfirm::All, UpdateArgs};
use baker::constants::DEFAULT_GENERATED_FILE_NAME;
use baker::generated;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tempfile::TempDir;
use test_log::test;
use walkdir::WalkDir;

/// Global mutex to serialise all tests that modify the process-wide CWD.
///
/// `std::env::set_current_dir` is a process-global operation.  Running tests
/// that change CWD concurrently produces races where one test's `current_dir()`
/// call returns the directory set by a different test.  Holding this lock for
/// the entire duration of each CWD-sensitive helper eliminates the race.
static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Run `baker generate` into a fresh temp dir and return the temp dir.
fn generate_into_tmp(template: &str, answers: Option<&str>) -> TempDir {
    let tmp = TempDir::new().unwrap();
    let args = GenerateArgs {
        template: template.to_string(),
        output_dir: tmp.path().to_path_buf(),
        force: true,
        answers: answers.map(|s| s.to_string()),
        answers_file: None,
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
        generated_file: None,
        conflict_style: None,
    };
    run(args).unwrap();
    tmp
}

/// Read the `.baker-generated.yaml` written inside `dir`.
///
/// Canonicalizes the path to handle macOS /var -> /private/var symlinks.
fn read_meta(dir: &Path) -> baker::generated::BakerGenerated {
    let canonical = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    generated::read(&canonical, DEFAULT_GENERATED_FILE_NAME).unwrap()
}

/// Run `baker update` from inside `output_dir` (sets CWD).
///
/// Holds `CWD_LOCK` for the entire call so parallel tests don't race.
/// Always restores CWD even if the update fails.
fn run_update_in(output_dir: &Path, extra_answers: Option<&str>) {
    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(output_dir).unwrap();
    let args = UpdateArgs {
        generated_file: None,
        answers: extra_answers.map(|s| s.to_string()),
        answers_file: None,
        conflict_style: None,
        dry_run: false,
        skip_confirms: vec![All],
        non_interactive: true,
    };
    let result = run_update(args);
    std::env::set_current_dir(original_dir).unwrap();
    result.unwrap();
}

/// Run `baker update` and expect it to return an error (returns the error message).
fn run_update_in_expect_err(output_dir: &Path) -> String {
    run_update_in_expect_err_with(output_dir, None, None)
}

/// Run `baker update` with custom answers/answers_file and expect an error.
fn run_update_in_expect_err_with(
    output_dir: &Path,
    answers: Option<&str>,
    answers_file: Option<&str>,
) -> String {
    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(output_dir).unwrap();
    let args = UpdateArgs {
        generated_file: None,
        answers: answers.map(|s| s.to_string()),
        answers_file: answers_file.map(std::path::PathBuf::from),
        conflict_style: None,
        dry_run: false,
        skip_confirms: vec![All],
        non_interactive: true,
    };
    let result = run_update(args);
    std::env::set_current_dir(original_dir).unwrap();
    format!("{}", result.unwrap_err())
}

/// When the template has not changed the update should exit early, and all
/// existing output files should remain byte-for-byte identical.
#[test]
fn update_local_template_unchanged_is_noop() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    run_update_in(output_dir.path(), None);

    assert_output_matches(output_dir.path(), "tests/expected/update_noop");
}

/// When the template content changes the update should re-render files.
#[test]
fn update_local_template_changed_rerenders_files() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Greetings, {{name}}!");

    run_update_in(output_dir.path(), None);

    assert_output_matches(output_dir.path(), "tests/expected/update_rerenders");
}

/// When the on-disk file was manually edited AND the template also changed,
/// the update should write conflict markers into the file.
#[test]
fn update_local_template_conflict_markers_on_user_edited_file() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    fs::write(
        output_dir.path().join("README.md"),
        "Hello, Alice!\nThis line was added by the user.\n",
    )
    .unwrap();

    write_template_file(template_dir.path(), "Hi there, {{name}}!");

    run_update_in(output_dir.path(), None);

    let content = fs::read_to_string(output_dir.path().join("README.md")).unwrap();
    assert!(
        content.contains("<<<<<<< current"),
        "conflict marker '<<<<<<< current' must be present; got:\n{content}"
    );
    assert!(
        content.contains("======="),
        "conflict separator '=======' must be present; got:\n{content}"
    );
    assert!(
        content.contains(">>>>>>> updated"),
        "conflict marker '>>>>>>> updated' must be present; got:\n{content}"
    );
    assert!(
        content.contains("This line was added by the user."),
        "user-added content must be preserved; got:\n{content}"
    );
    assert!(
        content.contains("Hi there, Alice!"),
        "updated template output must appear; got:\n{content}"
    );
}

// ---------------------------------------------------------------------------
// Local template — generated metadata is updated after a successful update
// ---------------------------------------------------------------------------

#[test]
fn update_local_template_metadata_is_refreshed() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    let before_meta = read_meta(output_dir.path());

    write_template_file(template_dir.path(), "Greetings, {{name}}!");

    run_update_in(output_dir.path(), None);

    let after_meta = read_meta(output_dir.path());

    match (&before_meta.template, &after_meta.template) {
        (
            baker::loader::TemplateSourceInfo::Filesystem { hash: old_hash, .. },
            baker::loader::TemplateSourceInfo::Filesystem { hash: new_hash, .. },
        ) => {
            assert_ne!(
                old_hash, new_hash,
                "template hash in metadata must change after template update"
            );
        }
        _ => panic!("expected Filesystem source info"),
    }
}

#[test]
fn update_fails_when_no_generated_file() {
    let empty_dir = TempDir::new().unwrap();
    let err = run_update_in_expect_err(empty_dir.path());
    assert!(
        err.contains("baker-generated")
            || err.contains("not found")
            || err.contains("Run"),
        "error message should guide the user to run generate first; got: {err}"
    );
}

#[test]
fn update_local_template_reuses_saved_answers() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Bob"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Dear {{name}}, welcome!");

    run_update_in(output_dir.path(), None);

    assert_output_matches(output_dir.path(), "tests/expected/update_reuses_answers");
}

#[test]
fn update_local_template_cli_answers_override_saved() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Bob"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Dear {{name}}, welcome!");

    run_update_in(output_dir.path(), Some(r#"{"name": "Carol"}"#));

    assert_output_matches(output_dir.path(), "tests/expected/update_cli_override");
}

#[test]
fn update_local_template_dry_run_no_changes() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Changed: {{name}}!");

    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(output_dir.path()).unwrap();
    let result = run_update(UpdateArgs {
        generated_file: None,
        answers: None,
        answers_file: None,
        conflict_style: None,
        dry_run: true,
        skip_confirms: vec![All],
        non_interactive: true,
    });
    std::env::set_current_dir(original_dir).unwrap();
    result.unwrap();

    assert_output_matches(output_dir.path(), "tests/expected/update_noop");
}

/// Uses the demo template for initial generation, then runs update without any
/// template changes.  The output files should remain identical.
#[test]
fn update_demo_template_noop() {
    let template_tmp = copy_to_tmp("examples/demo");
    let output_dir =
        generate_into_tmp(template_tmp.path().to_str().unwrap(), Some(DEMO_ANSWERS));

    run_update_in(output_dir.path(), None);

    assert_output_matches(output_dir.path(), "tests/expected/update_demo_noop");
}

/// Uses the demo template for initial generation, replaces its `README.md.baker.j2`
/// with the updated variant from `tests/templates/update_demo`, then runs update.
/// The rendered README changes, so the file must contain git-style conflict markers.
#[test]
fn update_demo_template_changed() {
    let template_tmp = copy_to_tmp("examples/demo");
    let output_dir =
        generate_into_tmp(template_tmp.path().to_str().unwrap(), Some(DEMO_ANSWERS));

    // Overwrite the README template with the changed version from tests/templates/update_demo.
    let v2_readme = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/templates/update_demo/README.md.baker.j2");
    fs::copy(v2_readme, template_tmp.path().join("README.md.baker.j2")).unwrap();

    run_update_in(output_dir.path(), None);

    assert_output_matches(output_dir.path(), "tests/expected/update_demo_changed");
}

/// Tests updating from a real git repository where we push a new commit.
///
/// Requires `cargo test --test update_integration_tests -- --ignored`
#[test]
#[ignore]
fn update_git_template_detects_new_commit() {
    // This test is intentionally left as a skeleton — the full implementation
    // mirrors gitea_integration_tests.rs and requires a running Gitea container.
    //
    // Steps the test would perform:
    //  1. Start a Gitea container (shared with gitea_integration_tests).
    //  2. Create a repo with baker.yaml and a template file.
    //  3. Run `baker generate` pointing at the Gitea URL.
    //  4. Verify the initial output and .baker-generated.yaml.
    //  5. Push a new commit to the repo (edit the template file).
    //  6. Run `baker update` from inside the generated output dir.
    //  7. Assert that the re-rendered file reflects the new commit.
    //  8. Verify that .baker-generated.yaml now stores the new commit SHA.
    unimplemented!(
        "git update test requires Docker; run with -- --ignored and a live Gitea instance"
    );
}

/// Tests that `baker update` exits early (no re-generation) when the git
/// repository's HEAD commit has not changed since the last `baker generate`.
///
/// Requires `cargo test --test update_integration_tests -- --ignored`
#[test]
#[ignore]
fn update_git_template_unchanged_commit_is_noop() {
    // Steps:
    //  1. Start Gitea, create repo, run `baker generate`.
    //  2. Run `baker update` immediately without pushing any new commits.
    //  3. Assert that the output files are unchanged and the tool prints
    //     "Template has not changed since last generation".
    unimplemented!(
        "git update test requires Docker; run with -- --ignored and a live Gitea instance"
    );
}
/// Passing malformed JSON via --answers should return a parse error.
#[test]
fn update_errors_on_malformed_answers_json() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Changed: {{name}}!");

    let err = run_update_in_expect_err_with(
        output_dir.path(),
        Some(r#"{"name":}"#), // malformed JSON
        None,
    );
    assert!(
        err.contains("JSON") || err.contains("parse") || err.contains("expected"),
        "expected a JSON parse error; got: {err}"
    );
}

/// Passing valid JSON that is not an object via --answers should return an error.
#[test]
fn update_errors_on_non_object_answers() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Changed: {{name}}!");

    let err = run_update_in_expect_err_with(
        output_dir.path(),
        Some(r#"["not", "an", "object"]"#),
        None,
    );
    assert!(
        err.contains("not an object") || err.contains("NotObject"),
        "expected an 'answers not object' error; got: {err}"
    );
}

/// Passing a path to a file with malformed JSON via --answers-file should error.
#[test]
fn update_errors_on_malformed_answers_file() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Changed: {{name}}!");

    let bad_file = output_dir.path().join("bad_answers.json");
    fs::write(&bad_file, r#"{ broken"#).unwrap();

    let err = run_update_in_expect_err_with(
        output_dir.path(),
        None,
        Some(bad_file.to_str().unwrap()),
    );
    assert!(
        err.contains("JSON") || err.contains("parse") || err.contains("expected"),
        "expected a JSON parse error; got: {err}"
    );
}

/// Passing a valid JSON array via --answers-file should error (not an object).
#[test]
fn update_errors_on_non_object_answers_file() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    write_template_file(template_dir.path(), "Changed: {{name}}!");

    let array_file = output_dir.path().join("array_answers.json");
    fs::write(&array_file, r#"[1, 2, 3]"#).unwrap();

    let err = run_update_in_expect_err_with(
        output_dir.path(),
        None,
        Some(array_file.to_str().unwrap()),
    );
    assert!(
        err.contains("not an object") || err.contains("NotObject"),
        "expected an 'answers not object' error; got: {err}"
    );
}

/// If .baker-generated.yaml has a version other than "1", update should fail.
#[test]
fn update_errors_on_unsupported_metadata_version() {
    let template_dir = TempDir::new().unwrap();
    create_simple_template(template_dir.path(), "Hello, {{name}}!");
    let answers = r#"{"name": "Alice"}"#;

    let output_dir =
        generate_into_tmp(template_dir.path().to_str().unwrap(), Some(answers));

    let meta_path = output_dir.path().join(DEFAULT_GENERATED_FILE_NAME);
    let content = fs::read_to_string(&meta_path).unwrap();
    let tampered = content
        .replace("version: '1'", "version: '2'")
        .replace("version: \"1\"", "version: \"2\"");
    fs::write(&meta_path, tampered).unwrap();

    let err = run_update_in_expect_err(output_dir.path());
    assert!(
        err.contains("Unsupported") || err.contains("version"),
        "expected unsupported version error; got: {err}"
    );
}

/// Answers for questions marked as `secret` should not appear in the generated
/// metadata file.
#[test]
fn generate_strips_secret_answers_from_metadata() {
    let template_dir = TempDir::new().unwrap();

    fs::write(
        template_dir.path().join("baker.yaml"),
        r#"schemaVersion: v1
questions:
  name:
    type: str
    help: Your name
    default: World
  password:
    type: str
    help: Enter password
    secret:
      confirm: false
"#,
    )
    .unwrap();
    fs::write(template_dir.path().join("README.md.baker.j2"), "Hello, {{name}}!")
        .unwrap();

    let tmp = TempDir::new().unwrap();
    let args = GenerateArgs {
        template: template_dir.path().to_str().unwrap().to_string(),
        output_dir: tmp.path().to_path_buf(),
        force: true,
        answers: Some(r#"{"name": "Alice", "password": "hunter2"}"#.to_string()),
        answers_file: None,
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
        generated_file: None,
        conflict_style: None,
    };
    run(args).unwrap();

    let meta = read_meta(tmp.path());
    assert_eq!(meta.answers.get("name").unwrap(), "Alice");
    assert!(
        meta.answers.get("password").is_none(),
        "password field should be stripped from metadata; got: {:?}",
        meta.answers
    );
}

const DEMO_ANSWERS: &str = r#"{"project_name": "demo", "project_author": "demo", "project_slug": "demo", "use_tests": true}"#;

/// Copy a directory tree from `src` (relative to the workspace root) into a
/// fresh `TempDir` and return it.
fn copy_to_tmp(src: &str) -> TempDir {
    let abs_src = Path::new(env!("CARGO_MANIFEST_DIR")).join(src);
    let tmp = TempDir::new().unwrap();
    copy_dir_all(&abs_src, tmp.path());
    tmp
}

/// Recursively copy all files and directories from `src` into `dst`.
fn copy_dir_all(src: &Path, dst: &Path) {
    for entry in WalkDir::new(src).min_depth(1).into_iter().filter_map(Result::ok) {
        let rel = entry.path().strip_prefix(src).unwrap();
        let dest = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest).unwrap();
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// Creates a minimal baker template directory:
///
/// ```
/// <dir>/
///   baker.yaml           (one `name` string question)
///   README.md.baker.j2   (renders `content` with `{{name}}`)
/// ```
fn create_simple_template(dir: &Path, content: &str) {
    fs::write(
        dir.join("baker.yaml"),
        "schemaVersion: v1\nquestions:\n  name:\n    type: str\n    help: What is your name?\n    default: World\n",
    )
    .unwrap();
    write_template_file(dir, content);
}

/// Overwrite the template file content without changing the baker.yaml.
fn write_template_file(dir: &Path, content: &str) {
    fs::write(dir.join("README.md.baker.j2"), content).unwrap();
}

/// Compare the output directory against an expected directory, ignoring
/// the `.baker-generated.yaml` metadata file (its content changes per run).
fn assert_output_matches(output_dir: &Path, expected_dir: &str) {
    let expected = Path::new(env!("CARGO_MANIFEST_DIR")).join(expected_dir);
    let _ = fs::remove_file(output_dir.join(DEFAULT_GENERATED_FILE_NAME));
    let result = dir_diff::is_different(output_dir, &expected);
    match result {
        Ok(different) => {
            if different {
                utils::print_dir_diff(output_dir, &expected);
                panic!("Directories differ. See above for details.");
            }
        }
        Err(e) => {
            panic!("Error comparing directories: {e}");
        }
    }
}
