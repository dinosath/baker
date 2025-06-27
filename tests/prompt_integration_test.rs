//! Integration test demonstrating the new prompt architecture
//!
//! This test shows how the new interface-based architecture makes
//! testing much easier compared to the old dialoguer-coupled approach.

use baker::config::{Question, Type, Validation};
use baker::prompt::automatic_impl::AutomaticPrompter;
use baker::prompt::{adapter::PromptAdapter, interface::*, PromptContext};
use serde_json::Value;

#[test]
fn test_prompt_adapter_with_automatic_provider() {
    // Create an automatic provider with predefined responses
    let auto_provider = AutomaticPrompter::new()
        .with_text_response("Enter your name:", "Test User")
        .with_confirmation_response("Do you agree?", true)
        .with_choice_response("Select option:", 1)
        .with_defaults("default_text", 0, false);

    let adapter = PromptAdapter::new(auto_provider);

    // Test text input
    let text_question = Question {
        help: "Enter your name:".to_string(),
        r#type: Type::Str,
        default: Value::String("".to_string()),
        choices: Vec::new(),
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        validation: Validation {
            condition: String::new(),
            error_message: "Invalid".to_string(),
        },
    };

    let default_text = Value::String("".to_string());
    let text_context =
        PromptContext::new(&text_question, &default_text, "Enter your name:");
    let text_result = adapter.prompt(&text_context).unwrap();
    assert_eq!(text_result, Value::String("Test User".to_string()));

    // Test confirmation
    let confirm_question = Question {
        help: "Do you agree?".to_string(),
        r#type: Type::Bool,
        default: Value::Bool(false),
        choices: Vec::new(),
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        validation: Validation {
            condition: String::new(),
            error_message: "Invalid".to_string(),
        },
    };

    let confirm_context =
        PromptContext::new(&confirm_question, &Value::Bool(false), "Do you agree?");
    let confirm_result = adapter.prompt(&confirm_context).unwrap();
    assert_eq!(confirm_result, Value::Bool(true));

    // Test choice selection
    let choice_question = Question {
        help: "Select option:".to_string(),
        r#type: Type::Str,
        default: Value::String("".to_string()),
        choices: vec![
            "Option A".to_string(),
            "Option B".to_string(),
            "Option C".to_string(),
        ],
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        validation: Validation {
            condition: String::new(),
            error_message: "Invalid".to_string(),
        },
    };

    let default_choice = Value::String("".to_string());
    let choice_context =
        PromptContext::new(&choice_question, &default_choice, "Select option:");
    let choice_result = adapter.prompt(&choice_context).unwrap();
    assert_eq!(choice_result, Value::String("Option B".to_string()));
}

#[test]
fn test_direct_interface_usage() {
    // Test using interfaces directly without adapter
    let provider = AutomaticPrompter::new()
        .with_text_response("Enter project name:", "my-awesome-project")
        .with_confirmation_response("Initialize git repo?", true);

    // Direct text prompt
    let text_config = TextPromptConfig {
        prompt: "Enter project name:".to_string(),
        default: None,
        secret: None,
    };
    let project_name = provider.prompt_text(&text_config).unwrap();
    assert_eq!(project_name, "my-awesome-project");

    // Direct confirmation prompt
    let confirm_config =
        ConfirmationConfig { prompt: "Initialize git repo?".to_string(), default: false };
    let init_git = provider.prompt_confirmation(&confirm_config).unwrap();
    assert_eq!(init_git, true);

    // Test with default values when no specific response is configured
    let unknown_text_config = TextPromptConfig {
        prompt: "Unknown prompt:".to_string(),
        default: Some("fallback".to_string()),
        secret: None,
    };
    let fallback_result = provider.prompt_text(&unknown_text_config).unwrap();
    assert_eq!(fallback_result, "fallback");
}

#[test]
fn test_multiple_choice_with_automatic_provider() {
    let provider = AutomaticPrompter::new()
        .with_multiple_choice_response("Select features:", vec![0, 2]);

    let multi_config = MultipleChoiceConfig {
        prompt: "Select features:".to_string(),
        choices: vec![
            "Feature A".to_string(),
            "Feature B".to_string(),
            "Feature C".to_string(),
        ],
        defaults: vec![false, false, false],
    };

    let selected_indices = provider.prompt_multiple_choice(&multi_config).unwrap();
    assert_eq!(selected_indices, vec![0, 2]);
}

#[test]
fn test_secret_text_prompt() {
    let provider =
        AutomaticPrompter::new().with_text_response("Enter password:", "secret123");

    let secret_config = TextPromptConfig {
        prompt: "Enter password:".to_string(),
        default: None,
        secret: Some(SecretConfig {
            confirm: true,
            mismatch_error: "Passwords don't match".to_string(),
        }),
    };

    let password = provider.prompt_text(&secret_config).unwrap();
    assert_eq!(password, "secret123");
}
