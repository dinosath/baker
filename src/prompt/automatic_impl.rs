//! Example of alternative prompt provider implementation
//!
//! This module shows how to create a custom prompt provider
//! that provides automatic responses without user interaction.

use super::interface::*;
use crate::error::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Automatic prompt provider that gives predefined responses
/// Useful for automation, testing, or CI/CD environments
#[derive(Debug)]
pub struct AutomaticPrompter {
    text_responses: HashMap<String, String>,
    choice_responses: HashMap<String, usize>,
    multiple_choice_responses: HashMap<String, Vec<usize>>,
    confirmation_responses: HashMap<String, bool>,
    structured_data_responses: HashMap<String, Value>,
    default_text: String,
    default_choice: usize,
    default_confirmation: bool,
}

impl AutomaticPrompter {
    pub fn new() -> Self {
        Self {
            text_responses: HashMap::new(),
            choice_responses: HashMap::new(),
            multiple_choice_responses: HashMap::new(),
            confirmation_responses: HashMap::new(),
            structured_data_responses: HashMap::new(),
            default_text: "auto".to_string(),
            default_choice: 0,
            default_confirmation: true,
        }
    }

    /// Add a predefined text response for a specific prompt
    pub fn with_text_response(mut self, prompt: &str, response: &str) -> Self {
        self.text_responses.insert(prompt.to_string(), response.to_string());
        self
    }

    /// Add a predefined choice response for a specific prompt
    pub fn with_choice_response(mut self, prompt: &str, choice_index: usize) -> Self {
        self.choice_responses.insert(prompt.to_string(), choice_index);
        self
    }

    /// Add a predefined multiple choice response for a specific prompt
    pub fn with_multiple_choice_response(
        mut self,
        prompt: &str,
        choice_indices: Vec<usize>,
    ) -> Self {
        self.multiple_choice_responses.insert(prompt.to_string(), choice_indices);
        self
    }

    /// Add a predefined confirmation response for a specific prompt
    pub fn with_confirmation_response(mut self, prompt: &str, response: bool) -> Self {
        self.confirmation_responses.insert(prompt.to_string(), response);
        self
    }

    /// Add a predefined structured data response for a specific prompt
    pub fn with_structured_data_response(
        mut self,
        prompt: &str,
        response: Value,
    ) -> Self {
        self.structured_data_responses.insert(prompt.to_string(), response);
        self
    }

    /// Set default values for when no specific response is configured
    pub fn with_defaults(
        mut self,
        default_text: &str,
        default_choice: usize,
        default_confirmation: bool,
    ) -> Self {
        self.default_text = default_text.to_string();
        self.default_choice = default_choice;
        self.default_confirmation = default_confirmation;
        self
    }
}

impl Default for AutomaticPrompter {
    fn default() -> Self {
        Self::new()
    }
}

impl TextPrompter for AutomaticPrompter {
    fn prompt_text(&self, config: &TextPromptConfig) -> Result<String> {
        let response = self
            .text_responses
            .get(&config.prompt)
            .cloned()
            .or_else(|| config.default.clone())
            .unwrap_or_else(|| self.default_text.clone());

        println!("Auto-answering text prompt '{}' with: '{}'", config.prompt, response);
        Ok(response)
    }
}

impl SingleChoicePrompter for AutomaticPrompter {
    fn prompt_single_choice(&self, config: &SingleChoiceConfig) -> Result<usize> {
        let response = self
            .choice_responses
            .get(&config.prompt)
            .copied()
            .or(config.default_index)
            .unwrap_or(self.default_choice);

        println!(
            "Auto-answering choice prompt '{}' with option {}: '{}'",
            config.prompt,
            response,
            config.choices.get(response).unwrap_or(&"<invalid>".to_string())
        );
        Ok(response)
    }
}

impl MultipleChoicePrompter for AutomaticPrompter {
    fn prompt_multiple_choice(
        &self,
        config: &MultipleChoiceConfig,
    ) -> Result<Vec<usize>> {
        let response = self
            .multiple_choice_responses
            .get(&config.prompt)
            .cloned()
            .unwrap_or_else(|| {
                // Default to selecting items that are marked as default
                config
                    .defaults
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &selected)| if selected { Some(i) } else { None })
                    .collect()
            });

        println!(
            "Auto-answering multiple choice prompt '{}' with options: {:?}",
            config.prompt, response
        );
        Ok(response)
    }
}

impl ConfirmationPrompter for AutomaticPrompter {
    fn prompt_confirmation(&self, config: &ConfirmationConfig) -> Result<bool> {
        let response = self
            .confirmation_responses
            .get(&config.prompt)
            .copied()
            .unwrap_or(config.default);

        println!("Auto-answering confirmation '{}' with: {}", config.prompt, response);
        Ok(response)
    }
}

impl StructuredDataPrompter for AutomaticPrompter {
    fn prompt_structured_data(&self, config: &StructuredDataConfig) -> Result<Value> {
        let response = self
            .structured_data_responses
            .get(&config.prompt)
            .cloned()
            .unwrap_or_else(|| config.default_value.clone());

        println!(
            "Auto-answering structured data prompt '{}' with default value",
            config.prompt
        );
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automatic_prompter() {
        let prompter = AutomaticPrompter::new()
            .with_text_response("Enter name:", "John Doe")
            .with_choice_response("Choose option:", 1)
            .with_confirmation_response("Are you sure?", false);

        // Test text prompt
        let text_config = TextPromptConfig {
            prompt: "Enter name:".to_string(),
            default: None,
            secret: None,
        };
        assert_eq!(prompter.prompt_text(&text_config).unwrap(), "John Doe");

        // Test choice prompt
        let choice_config = SingleChoiceConfig {
            prompt: "Choose option:".to_string(),
            choices: vec!["Option A".to_string(), "Option B".to_string()],
            default_index: None,
        };
        assert_eq!(prompter.prompt_single_choice(&choice_config).unwrap(), 1);

        // Test confirmation prompt
        let confirmation_config =
            ConfirmationConfig { prompt: "Are you sure?".to_string(), default: true };
        assert_eq!(prompter.prompt_confirmation(&confirmation_config).unwrap(), false);
    }

    #[test]
    fn test_automatic_prompter_defaults() {
        let prompter = AutomaticPrompter::new().with_defaults("default_text", 2, false);

        // Test with unknown prompt - should use defaults
        let text_config = TextPromptConfig {
            prompt: "Unknown prompt:".to_string(),
            default: None,
            secret: None,
        };
        assert_eq!(prompter.prompt_text(&text_config).unwrap(), "default_text");

        let choice_config = SingleChoiceConfig {
            prompt: "Unknown choice:".to_string(),
            choices: vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
            ],
            default_index: None,
        };
        assert_eq!(prompter.prompt_single_choice(&choice_config).unwrap(), 2);
    }
}
