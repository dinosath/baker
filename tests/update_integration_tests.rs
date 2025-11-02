use baker::cli::metadata::BakerMeta;
use baker::cli::runner::run_update;
use baker::cli::{run, Args, SkipConfirm::All};
use serde_json::Value;
use tempfile::TempDir;

/// End-to-end update test using an existing template fixture (single_choice):
/// 1. Copy existing template fixture into an isolated temp directory.
/// 2. Generate project with an overridden answer for `favourite_language`.
/// 3. Modify the copied template by adding a new question + template file.
/// 4. Run update with only the new answer (template path omitted, resolved via metadata).
/// 5. Verify original answer retained, new file generated, and metadata merged.
#[test]
fn test_update() {
    let temp_ws = TempDir::new().unwrap();
    let template_root = temp_ws.path().join("template");
    std::fs::create_dir_all(&template_root).unwrap();

    // Copy existing template fixture (single_choice) into temp template_root to avoid mutating repo files
    let fixture_root = std::path::Path::new("tests/templates/single_choice");
    for entry in std::fs::read_dir(fixture_root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            std::fs::copy(&path, template_root.join(path.file_name().unwrap())).unwrap();
        }
    }

    // Initial generation with a custom favourite_language answer
    let output_dir = temp_ws.path().join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let copy_args = Args {
        template: template_root.to_string_lossy().to_string(),
        output_dir: output_dir.clone(),
        force: true,
        verbose: 0,
        answers: Some(r#"{"favourite_language":"Go"}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };
    run(copy_args).expect("initial generation should succeed");

    // Validate initial output
    let readme_path = output_dir.join("README.md");
    assert!(readme_path.exists(), "README.md should be generated");
    let readme_content_initial = std::fs::read_to_string(&readme_path).unwrap();
    assert!(readme_content_initial.contains("Go"));

    let meta_before = BakerMeta::load(&output_dir).unwrap().expect("metadata should exist after initial run");
    assert_eq!(meta_before.template_source, template_root.to_string_lossy());
    assert_eq!(meta_before.answers["favourite_language"], Value::String("Go".into()));

    // Update: add new question + file in copied template
    let updated_config = r#"schemaVersion: v1
questions:
  favourite_language:
    type: str
    help: What is your favorite programming language?
    default: Rust
    choices:
      - Python
      - Rust
      - Go
      - TypeScript
  project_description:
    type: str
    help: Description?
"#;
    std::fs::write(template_root.join("baker.yaml"), updated_config).unwrap();
    std::fs::write(
        template_root.join("DESCRIPTION.md.baker.j2"),
        "Desc: {{ project_description }}\n",
    ).unwrap();

    // Run update supplying only the new answer. Empty template triggers metadata lookup.
    let update_args = Args {
        template: "".to_string(),
        output_dir: output_dir.clone(),
        force: false,
        verbose: 0,
        answers: Some(r#"{"project_description":"Awesome project"}"#.to_string()),
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };
    run_update(update_args).expect("update should succeed");

    // README should still contain original favourite_language answer
    let readme_content_after = std::fs::read_to_string(&readme_path).unwrap();
    assert!(readme_content_after.contains("Go"), "Existing answer should be preserved after update");

    // New DESCRIPTION.md should be rendered with new answer
    let desc_path = output_dir.join("DESCRIPTION.md");
    assert!(desc_path.exists(), "DESCRIPTION.md should be generated during update");
    let desc_content = std::fs::read_to_string(&desc_path).unwrap();
    assert!(desc_content.contains("Awesome project"));

    // Metadata should now include both answers
    let meta_after = BakerMeta::load(&output_dir).unwrap().expect("metadata should still exist");
    assert_eq!(meta_after.template_source, template_root.to_string_lossy());
    assert_eq!(meta_after.answers["favourite_language"], Value::String("Go".into()));
    assert_eq!(meta_after.answers["project_description"], Value::String("Awesome project".into()));
}
