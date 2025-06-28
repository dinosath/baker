//! Pure interfaces for prompting without external dependencies
//!
//! This module defines abstract interfaces for different types of user prompts.
//! These interfaces are independent of any specific UI library implementation.

use crate::error::Result;
use serde_json::Value;

/// Configuration for text input prompts
#[derive(Debug, Clone)]
pub struct TextPromptConfig {
    pub prompt: String,
    pub default: Option<String>,
    pub secret: Option<SecretConfig>,
}

/// Configuration for password/secret input
#[derive(Debug, Clone)]
pub struct SecretConfig {
    pub confirm: bool,
    pub mismatch_error: String,
}

/// Configuration for single choice selection
#[derive(Debug, Clone)]
pub struct SingleChoiceConfig {
    pub prompt: String,
    pub choices: Vec<String>,
    pub default_index: Option<usize>,
}

/// Configuration for multiple choice selection
#[derive(Debug, Clone)]
pub struct MultipleChoiceConfig {
    pub prompt: String,
    pub choices: Vec<String>,
    pub defaults: Vec<bool>,
}

/// Configuration for boolean confirmation
#[derive(Debug, Clone)]
pub struct ConfirmationConfig {
    pub prompt: String,
    pub default: bool,
}

/// Configuration for structured data input (JSON/YAML)
#[derive(Debug, Clone)]
pub struct StructuredDataConfig {
    pub prompt: String,
    pub default_value: Value,
    pub is_yaml: bool,
    pub file_extension: String,
}

/// Abstract interface for text input prompts
pub trait TextPrompter {
    fn prompt_text(&self, config: &TextPromptConfig) -> Result<String>;
}

/// Abstract interface for single choice selection
pub trait SingleChoicePrompter {
    fn prompt_single_choice(&self, config: &SingleChoiceConfig) -> Result<usize>;
}

/// Abstract interface for multiple choice selection
pub trait MultipleChoicePrompter {
    fn prompt_multiple_choice(&self, config: &MultipleChoiceConfig)
        -> Result<Vec<usize>>;
}

/// Abstract interface for boolean confirmation
pub trait ConfirmationPrompter {
    fn prompt_confirmation(&self, config: &ConfirmationConfig) -> Result<bool>;
}

/// Abstract interface for structured data input
pub trait StructuredDataPrompter {
    fn prompt_structured_data(&self, config: &StructuredDataConfig) -> Result<Value>;
}

/// Combined interface that provides all prompt types
pub trait PromptProvider:
    TextPrompter
    + SingleChoicePrompter
    + MultipleChoicePrompter
    + ConfirmationPrompter
    + StructuredDataPrompter
{
}

// Blanket implementation for any type that implements all prompt interfaces
impl<T> PromptProvider for T where
    T: TextPrompter
        + SingleChoicePrompter
        + MultipleChoicePrompter
        + ConfirmationPrompter
        + StructuredDataPrompter
{
}
