use baker::cli::{run, Args, SkipConfirm::All};
use test_log::test;

#[test]
fn test_single_choice_question() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/single_choice".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(r#"{"favourite_language": "Go"}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(
        !dir_diff::is_different(tmp_dir.path(), "tests/expected/single_choice").unwrap()
    );
}

#[test]
fn test_multiple_choice_question() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/multiple_choice".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(r#"{"favourite_languages": ["Go", "TypeScript"]}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/multiple_choice")
        .unwrap());
}

#[test]
fn test_yaml_complex_type() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let yaml_config = r#"
    {
        "environments": {
            "development": {
                "url": "http://localhost:8000",
                "debug": true
            },
            "production": {
                "url": "https://staging.example.com",
                "debug": false
            }
        }
    }"#;
    let args = Args {
        template: "tests/templates/yaml_type".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(yaml_config.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/yaml_type").unwrap());
}

#[test]
fn test_conditional_questions_python() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/conditional_questions".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(
            r#"{"language": "Python", "py_framework": "Django", "include_docker": true}"#
                .to_string(),
        ),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(
        tmp_dir.path(),
        "tests/expected/conditional_questions_python"
    )
    .unwrap());
}

#[test]
fn test_conditional_questions_rust() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/conditional_questions".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(r#"{"language": "Rust"}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(
        tmp_dir.path(),
        "tests/expected/conditional_questions_rust"
    )
    .unwrap());
}

#[test]
fn test_validation_patterns() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/validation".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(
            r#"{"project_name": "My Test Project", "age": "25", "email": "test@example.com"}"#
                .to_string(),
        ),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/validation").unwrap());
}

#[test]
fn test_builtin_filters() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/builtin_filters".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(
            r#"{"project_name": "Example Project", "project_slug": "example_project", "class_name": "ExampleProject", "table_name": "example_projects", "constant_name": "EXAMPLE_PROJECT"}"#
                .to_string(),
        ),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/builtin_filters")
        .unwrap());
}

#[test]
fn test_templated_filenames_with_database() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/templated_filenames".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(
            r#"{"project_name": "my awesome project", "project_slug": "my_awesome_project", "project_class": "MyAwesomeProject", "use_database": true}"#.to_string(),
        ),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(
        tmp_dir.path(),
        "tests/expected/templated_filenames"
    )
    .unwrap());
}

#[test]
fn test_templated_filenames_without_database() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/templated_filenames".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(
            r#"{"project_name": "my awesome project", "project_slug": "my_awesome_project", "project_class": "MyAwesomeProject", "use_database": false}"#
                .to_string(),
        ),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(
        tmp_dir.path(),
        "tests/expected/templated_filenames_no_db"
    )
    .unwrap());
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_custom_hooks() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/custom_hooks".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(r#"{"username": "testuser", "project_type": "web"}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(
        !dir_diff::is_different(tmp_dir.path(), "tests/expected/custom_hooks").unwrap()
    );
}

#[test]
#[cfg(target_os = "macos")]
fn test_platform_variables() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/platform_variables".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(r#"{"project_name": "cross-platform-app"}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(tmp_dir.path(), "tests/expected/platform_variables")
        .unwrap());
}

#[test]
fn test_non_interactive_mode_with_defaults() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/builtin_filters".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: None, // Test default values being used
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();

    // Check that the file was created with default values
    let output_file = tmp_dir.path().join("output.md");
    assert!(output_file.exists());
    let content = std::fs::read_to_string(output_file).unwrap();
    assert!(content.contains("Example Project"));
}

#[test]
fn test_template_with_different_suffix() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "tests/templates/different_template_suffix".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: None,
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();
    assert!(!dir_diff::is_different(
        tmp_dir.path(),
        "tests/expected/different_template_suffix"
    )
    .unwrap());
}

#[test]
fn test_nested_answer_context() {
    // Test that previous answers are available in later questions
    let tmp_dir = tempfile::tempdir().unwrap();
    let args = Args {
        template: "examples/demo".to_string(),
        output_dir: tmp_dir.path().to_path_buf(),
        force: true,
        verbose: Some(None),
        answers: Some(r#"{"project_name": "Test Project", "project_author": "Test Author", "project_slug": "test_project", "use_tests": true}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
    };
    run(args).unwrap();

    // Verify the content uses interpolated values correctly
    let readme_file = tmp_dir.path().join("CONTRIBUTING.md");
    assert!(readme_file.exists());
    let content = std::fs::read_to_string(readme_file).unwrap();
    assert!(content.contains("Test Project"));
    assert!(content.contains("Test Author"));
}
