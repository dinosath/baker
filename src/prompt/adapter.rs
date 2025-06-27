//! Adapter layer for converting between old and new prompt systems
//!
//! This module provides backward compatibility by adapting the existing
//! prompt system to use the new interface-based architecture.

use super::interface::{
    ConfirmationConfig, MultipleChoiceConfig, PromptProvider, SecretConfig,
    SingleChoiceConfig, StructuredDataConfig, TextPromptConfig,
};
use crate::{
    config::{IntoQuestionType, QuestionType},
    error::Result,
    prompt::PromptContext,
};
use serde_json::Value;

/// Adapter that converts PromptContext to interface configurations
pub struct PromptAdapter<P: PromptProvider> {
    provider: P,
}

impl<P: PromptProvider> PromptAdapter<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Main prompt method that delegates to appropriate interface
    pub fn prompt(&self, prompt_context: &PromptContext) -> Result<Value> {
        match prompt_context.question.into_question_type() {
            QuestionType::Text => self.prompt_text(prompt_context),
            QuestionType::SingleChoice => self.prompt_single_choice(prompt_context),
            QuestionType::MultipleChoice => self.prompt_multiple_choice(prompt_context),
            QuestionType::Boolean => self.prompt_confirmation(prompt_context),
            QuestionType::Json => self.prompt_structured_data(prompt_context, false),
            QuestionType::Yaml => self.prompt_structured_data(prompt_context, true),
        }
    }

    fn prompt_text(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_text_config(prompt_context);
        let result = self.provider.prompt_text(&config)?;
        Ok(Value::String(result))
    }

    fn prompt_single_choice(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_single_choice_config(prompt_context);
        let selection_index = self.provider.prompt_single_choice(&config)?;
        let selected_choice = &prompt_context.question.choices[selection_index];
        Ok(Value::String(selected_choice.clone()))
    }

    fn prompt_multiple_choice(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_multiple_choice_config(prompt_context);
        let indices = self.provider.prompt_multiple_choice(&config)?;

        let selected: Vec<Value> = indices
            .iter()
            .map(|&i| Value::String(prompt_context.question.choices[i].clone()))
            .collect();

        Ok(Value::Array(selected))
    }

    fn prompt_confirmation(&self, prompt_context: &PromptContext) -> Result<Value> {
        let config = self.create_confirmation_config(prompt_context);
        let result = self.provider.prompt_confirmation(&config)?;
        Ok(Value::Bool(result))
    }

    fn prompt_structured_data(
        &self,
        prompt_context: &PromptContext,
        is_yaml: bool,
    ) -> Result<Value> {
        let config = self.create_structured_data_config(prompt_context, is_yaml);
        self.provider.prompt_structured_data(&config)
    }

    // Configuration creation methods

    fn create_text_config(&self, prompt_context: &PromptContext) -> TextPromptConfig {
        let default = self.value_to_default_string(prompt_context.default);
        let secret = prompt_context.question.secret.as_ref().map(|s| SecretConfig {
            confirm: s.confirm,
            mismatch_error: if s.mistmatch_err.is_empty() {
                "Mismatch".to_string()
            } else {
                s.mistmatch_err.clone()
            },
        });

        TextPromptConfig {
            prompt: prompt_context.help.to_string(),
            default: if default.is_empty() { None } else { Some(default) },
            secret,
        }
    }

    fn create_single_choice_config(
        &self,
        prompt_context: &PromptContext,
    ) -> SingleChoiceConfig {
        let default_index = self.find_default_choice_index(
            &prompt_context.question.choices,
            prompt_context.default,
        );

        SingleChoiceConfig {
            prompt: prompt_context.help.to_string(),
            choices: prompt_context.question.choices.clone(),
            default_index: if default_index > 0 { Some(default_index) } else { None },
        }
    }

    fn create_multiple_choice_config(
        &self,
        prompt_context: &PromptContext,
    ) -> MultipleChoiceConfig {
        let default_strings = self.extract_string_array(prompt_context.default);
        let defaults = self
            .create_choice_defaults(&prompt_context.question.choices, &default_strings);

        MultipleChoiceConfig {
            prompt: prompt_context.help.to_string(),
            choices: prompt_context.question.choices.clone(),
            defaults,
        }
    }

    fn create_confirmation_config(
        &self,
        prompt_context: &PromptContext,
    ) -> ConfirmationConfig {
        let default = prompt_context.default.as_bool().unwrap_or(false);

        ConfirmationConfig { prompt: prompt_context.help.to_string(), default }
    }

    fn create_structured_data_config(
        &self,
        prompt_context: &PromptContext,
        is_yaml: bool,
    ) -> StructuredDataConfig {
        let file_extension = if is_yaml { ".yaml" } else { ".json" };

        StructuredDataConfig {
            prompt: prompt_context.help.to_string(),
            default_value: prompt_context.default.clone(),
            is_yaml,
            file_extension: file_extension.to_string(),
        }
    }

    // Helper methods

    fn value_to_default_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => value.to_string(),
        }
    }

    fn find_default_choice_index(
        &self,
        choices: &[String],
        default_value: &Value,
    ) -> usize {
        match default_value {
            Value::String(default_str) => {
                choices.iter().position(|choice| choice == default_str).unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn extract_string_array(&self, value: &Value) -> Vec<String> {
        match value {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn create_choice_defaults(
        &self,
        choices: &[String],
        default_strings: &[String],
    ) -> Vec<bool> {
        choices.iter().map(|choice| default_strings.contains(choice)).collect()
    }
}
