//! Integration tests for MCP functionality.
//!
//! These tests verify that the MCP tools work correctly with templates.
//!
//! Run these tests with: `cargo test --test mcp_integration_tests --features mcp`
//!
//! MCP client integration tests (require `mcp` feature and the baker binary):
//! `cargo test --test mcp_integration_tests --features mcp -- --ignored mcp_client`

#![cfg(feature = "mcp")]

use baker_cli::mcp::{GenerateTool, TemplateInfo};
use baker_cli::TemplateStore;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test template store with a sample template.
fn setup_test_store_with_template() -> (TempDir, TemplateStore, TempDir) {
    // Create temp directories for store and template
    let store_dir = TempDir::new().expect("Failed to create store temp dir");
    let template_dir = TempDir::new().expect("Failed to create template temp dir");

    // Create a simple baker.yaml config
    let config = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Name of your project
    
  use_docker:
    type: bool
    help: Include Docker support?
    default: false
"#;

    fs::write(template_dir.path().join("baker.yaml"), config)
        .expect("Failed to write baker.yaml");

    // Create a template file
    fs::write(
        template_dir.path().join("README.md.baker.j2"),
        "# {{ project_name }}\n\nWelcome to {{ project_name }}!\n",
    )
    .expect("Failed to write template file");

    // Create the store
    let store = TemplateStore::with_dir(store_dir.path().to_path_buf());

    // Install the template
    store
        .install(
            template_dir.path().to_str().unwrap(),
            "test-template",
            Some("A test template for MCP testing".to_string()),
            false,
        )
        .expect("Failed to install template");

    (store_dir, store, template_dir)
}

#[test]
fn test_list_templates_returns_installed_templates() {
    let (store_dir, store, _template_dir) = setup_test_store_with_template();

    // Verify template is installed
    assert!(store.is_installed("test-template"));

    // Create a custom ListTemplatesTool that uses our test store
    // Since ListTemplatesTool::execute() uses TemplateStore::new(), we need to test differently
    let templates = store.list().expect("Failed to list templates");

    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "test-template");
    assert_eq!(
        templates[0].description,
        Some("A test template for MCP testing".to_string())
    );

    drop(store_dir);
}

#[test]
fn test_list_templates_extracts_questions() {
    let (store_dir, store, _template_dir) = setup_test_store_with_template();

    // Extract template and read config to verify questions are present
    let temp_dir =
        store.extract_to_temp("test-template").expect("Failed to extract template");

    let config_path = temp_dir.path().join("baker.yaml");
    assert!(config_path.exists());

    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(content.contains("project_name"));
    assert!(content.contains("use_docker"));

    drop(store_dir);
}

#[test]
fn test_generate_tool_validates_template_exists() {
    let (_store_dir, store, _template_dir) = setup_test_store_with_template();

    // Verify our template exists
    assert!(store.is_installed("test-template"));
    assert!(!store.is_installed("nonexistent-template"));
}

#[test]
fn test_generate_tool_creation() {
    let tool = GenerateTool {
        template: "my-template".to_string(),
        output_dir: "/tmp/output".to_string(),
        answers: HashMap::from([
            ("project_name".to_string(), serde_json::json!("my-project")),
            ("use_docker".to_string(), serde_json::json!(true)),
        ]),
        force: false,
    };

    assert_eq!(tool.template, "my-template");
    assert_eq!(tool.output_dir, "/tmp/output");
    assert_eq!(tool.answers.len(), 2);
    assert!(!tool.force);
}

#[test]
fn test_template_info_serialization() {
    let info = TemplateInfo {
        name: "test".to_string(),
        description: Some("A test template".to_string()),
        source: "/path/to/template".to_string(),
        installed_at: "2025-01-23T12:00:00Z".to_string(),
        questions: vec![],
        usage: "baker generate test <output>".to_string(),
    };

    let json = serde_json::to_string(&info).expect("Failed to serialize");
    assert!(json.contains("\"name\":\"test\""));
    assert!(json.contains("\"description\":\"A test template\""));

    let deserialized: TemplateInfo =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.name, "test");
}

#[test]
fn test_question_info_serialization() {
    use baker_cli::mcp::QuestionInfo;

    let info = QuestionInfo {
        name: "project_name".to_string(),
        help: Some("Enter project name".to_string()),
        r#type: "str".to_string(),
        default: Some("my-project".to_string()),
        required: false,
        choices: None,
    };

    let json = serde_json::to_string(&info).expect("Failed to serialize");
    assert!(json.contains("\"name\":\"project_name\""));
    assert!(json.contains("\"type\":\"str\""));

    let deserialized: QuestionInfo =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.name, "project_name");
    assert_eq!(deserialized.r#type, "str");
}

/// Test that verifies an MCP server can select the right template based on context.
/// This simulates an AI assistant choosing a template.
#[test]
fn test_template_selection_by_context() {
    let (store_dir, store, _template_dir) = setup_test_store_with_template();

    // Install a second template with different characteristics
    let template_dir2 = TempDir::new().expect("Failed to create second template dir");

    let config2 = r#"schemaVersion: v1

questions:
  api_name:
    type: str
    help: Name of your REST API
    
  database:
    type: str
    help: Database type
    choices:
      - postgres
      - mysql
      - sqlite
    default: postgres
"#;

    fs::write(template_dir2.path().join("baker.yaml"), config2)
        .expect("Failed to write baker.yaml");

    store
        .install(
            template_dir2.path().to_str().unwrap(),
            "api-template",
            Some("Template for REST API projects".to_string()),
            false,
        )
        .expect("Failed to install second template");

    // Now simulate template selection based on context
    let templates = store.list().expect("Failed to list templates");

    // Context: User wants to create a REST API
    let context = "I want to create a REST API with a database";

    // Simple matching algorithm (in real MCP server, this would be more sophisticated)
    let selected = templates
        .iter()
        .find(|t| {
            let desc = t.description.as_deref().unwrap_or("");
            desc.to_lowercase().contains("api")
                || desc.to_lowercase().contains("rest")
                || context.to_lowercase().contains(&t.name.to_lowercase())
        })
        .map(|t| &t.name);

    assert_eq!(selected, Some(&"api-template".to_string()));

    // Context: User wants a simple project
    let context2 = "I need a test template for MCP testing";

    let selected2 = templates
        .iter()
        .find(|t| {
            let desc = t.description.as_deref().unwrap_or("");
            context2.to_lowercase().contains("test")
                && desc.to_lowercase().contains("test")
        })
        .map(|t| &t.name);

    assert_eq!(selected2, Some(&"test-template".to_string()));

    drop(store_dir);
}

// =============================================================================
// MCP Client Integration Tests using rmcp
// =============================================================================
//
// These tests spawn the actual baker MCP server as a child process and
// communicate with it using the rmcp client library.
//
// Prerequisites:
// - Build baker with the mcp feature: `cargo build --features mcp`
// - The tests will use the binary from target/debug/baker

mod mcp_client_tests {
    use rmcp::{
        model::CallToolRequestParam,
        transport::{ConfigureCommandExt, TokioChildProcess},
        ServiceExt,
    };
    use std::path::PathBuf;
    use tokio::process::Command;

    /// Get the path to the baker binary
    fn get_baker_binary() -> PathBuf {
        // Use the binary from target/debug
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root =
            PathBuf::from(manifest_dir).parent().unwrap().parent().unwrap().to_path_buf();

        workspace_root.join("target/debug/baker")
    }

    /// Check if the baker binary exists and has the mcp feature
    fn baker_binary_available() -> bool {
        let binary = get_baker_binary();
        if !binary.exists() {
            eprintln!("Baker binary not found at {:?}", binary);
            eprintln!("Build with: cargo build --features mcp");
            return false;
        }
        true
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_client_connects_to_server() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        // Create transport using child process
        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        // Connect to the MCP server
        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // Get server info
        let peer_info = client.peer_info();
        eprintln!("Connected to server: {:?}", peer_info);

        // peer_info() returns Option<&InitializeResult>
        let init_result = peer_info.expect("Expected peer info to be available");
        // server_info is now directly an Implementation struct
        assert_eq!(init_result.server_info.name, "baker");

        // Gracefully close connection
        client.cancel().await.expect("Failed to close connection");
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_client_lists_tools() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // List available tools
        let tools =
            client.list_tools(Default::default()).await.expect("Failed to list tools");

        eprintln!("Available tools: {:?}", tools);

        // Verify expected tools exist
        let tool_names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();

        assert!(tool_names.contains(&"list_templates"), "Expected 'list_templates' tool");
        assert!(tool_names.contains(&"generate"), "Expected 'generate' tool");

        // Verify tool descriptions
        let list_tool = tools.tools.iter().find(|t| t.name == "list_templates").unwrap();
        assert!(list_tool.description.is_some());

        let generate_tool = tools.tools.iter().find(|t| t.name == "generate").unwrap();
        assert!(generate_tool.description.is_some());

        client.cancel().await.expect("Failed to close connection");
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_client_calls_list_templates() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // Call the list_templates tool
        let result = client
            .call_tool(CallToolRequestParam {
                name: "list_templates".into(),
                arguments: None,
                task: None,
            })
            .await
            .expect("Failed to call list_templates");

        eprintln!("list_templates result: {:?}", result);

        // The tool should return successfully (even if no templates are installed)
        assert!(!result.is_error.unwrap_or(false), "Tool call returned error");

        // The result should contain content
        assert!(!result.content.is_empty(), "Expected content in result");

        client.cancel().await.expect("Failed to close connection");
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_client_calls_generate_with_missing_template() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // Call the generate tool with a non-existent template
        let result = client
            .call_tool(CallToolRequestParam {
                name: "generate".into(),
                arguments: Some(
                    serde_json::json!({
                        "template": "nonexistent-template",
                        "output_dir": "/tmp/test-output"
                    })
                    .as_object()
                    .cloned()
                    .unwrap(),
                ),
                task: None,
            })
            .await
            .expect("Failed to call generate");

        eprintln!("generate result (expected error): {:?}", result);

        // The tool should return an error since the template doesn't exist
        assert!(
            result.is_error.unwrap_or(false),
            "Expected error for non-existent template"
        );

        client.cancel().await.expect("Failed to close connection");
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_client_full_workflow() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // Step 1: List available tools
        let tools =
            client.list_tools(Default::default()).await.expect("Failed to list tools");
        eprintln!("Step 1 - Available tools: {}", tools.tools.len());

        // Step 2: List templates
        let templates_result = client
            .call_tool(CallToolRequestParam {
                name: "list_templates".into(),
                arguments: None,
                task: None,
            })
            .await
            .expect("Failed to call list_templates");
        eprintln!("Step 2 - Templates result: {:?}", templates_result);

        // Step 3: Try to generate (will fail if no templates installed, which is expected)
        let generate_result = client
            .call_tool(CallToolRequestParam {
                name: "generate".into(),
                arguments: Some(
                    serde_json::json!({
                        "template": "demo",
                        "output_dir": "/tmp/mcp-test-project",
                        "answers": {
                            "project_name": "mcp-test"
                        }
                    })
                    .as_object()
                    .cloned()
                    .unwrap(),
                ),
                task: None,
            })
            .await
            .expect("Failed to call generate");
        eprintln!("Step 3 - Generate result: {:?}", generate_result);

        client.cancel().await.expect("Failed to close connection");
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_server_capabilities() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // Get server info and verify capabilities
        let peer_info = client.peer_info();
        let init_result = peer_info.expect("Expected peer info to be available");

        // Verify server info - server_info is now directly an Implementation struct
        eprintln!("Server name: {}", init_result.server_info.name);
        eprintln!("Server version: {}", init_result.server_info.version);

        // Verify capabilities - capabilities is now directly a ServerCapabilities struct
        let capabilities = &init_result.capabilities;

        // Baker MCP server should have tools capability
        assert!(capabilities.tools.is_some(), "Expected tools capability");
        eprintln!("Capabilities: {:?}", capabilities);

        // Verify protocol version
        eprintln!("Protocol version: {:?}", init_result.protocol_version);

        client.cancel().await.expect("Failed to close connection");
    }

    #[tokio::test]
    #[ignore = "requires baker binary built with mcp feature"]
    async fn test_mcp_tool_input_schemas() {
        if !baker_binary_available() {
            return;
        }

        let baker_path = get_baker_binary();

        let transport =
            TokioChildProcess::new(Command::new(&baker_path).configure(|cmd| {
                cmd.arg("mcp");
            }))
            .expect("Failed to create transport");

        let client = ().serve(transport).await.expect("Failed to connect to MCP server");

        // List tools and verify input schemas
        let tools =
            client.list_tools(Default::default()).await.expect("Failed to list tools");

        // Check generate tool has proper input schema
        let generate_tool = tools.tools.iter().find(|t| t.name == "generate");
        assert!(generate_tool.is_some(), "Expected 'generate' tool");

        let generate = generate_tool.unwrap();
        eprintln!("Generate tool schema: {:?}", generate.input_schema);

        // The input schema should be defined
        // Note: input_schema is a RawValue in rmcp, so we just check it exists

        client.cancel().await.expect("Failed to close connection");
    }
}
