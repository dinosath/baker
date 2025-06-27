//! Interactive dialog utilities for user input
//!
//! This module provides a modular approach to handling different types of user prompts
//! including text input, choices, confirmations, and structured data.

use crate::{
    config::{IntoQuestionType, Question, QuestionType, Type},
    error::Result,
};

pub mod choice;
pub mod confirmation;
pub mod structured;
pub mod text;

use choice::{MultipleChoicePrompter, SingleChoicePrompter};
use confirmation::ConfirmationPrompter;
use structured::StructuredDataPrompter;
use text::TextPrompter;

/// Common interface for all prompt types
pub trait Prompter<'a> {
    fn prompt(&self, prompt_context: &PromptContext<'a>) -> Result<serde_json::Value>;
}

/// Context for prompting operations
pub struct PromptContext<'a> {
    pub question: &'a Question,
    pub default: &'a serde_json::Value,
    pub help: &'a str,
}

impl<'a> PromptContext<'a> {
    pub fn new(
        question: &'a Question,
        default: &'a serde_json::Value,
        help: &'a str,
    ) -> Self {
        Self { question, default, help }
    }
}

/// Main entry point for asking questions
pub fn ask_question(
    question: &Question,
    default: &serde_json::Value,
    help: String,
) -> Result<serde_json::Value> {
    let context = PromptContext::new(question, default, &help);

    match question.into_question_type() {
        QuestionType::MultipleChoice => MultipleChoicePrompter.prompt(&context),
        QuestionType::Boolean => ConfirmationPrompter.prompt(&context),
        QuestionType::SingleChoice => SingleChoicePrompter.prompt(&context),
        QuestionType::Text => TextPrompter.prompt(&context),
        QuestionType::Json | QuestionType::Yaml => {
            StructuredDataPrompter::new(question.into_question_type()).prompt(&context)
        }
    }
}

/// Simple confirmation function for backward compatibility
pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    if skip {
        return Ok(true);
    }

    let question = Question {
        help: prompt,
        r#type: Type::Bool,
        default: serde_json::Value::Bool(false),
        choices: Vec::new(),
        multiselect: false,
        secret: None,
        ask_if: String::new(),
        schema: None,
        validation: crate::config::Validation {
            condition: String::new(),
            error_message: "Invalid answer".to_string(),
        },
    };

    let default_value = serde_json::Value::Bool(false);
    let context = PromptContext::new(&question, &default_value, &question.help);
    let result = ConfirmationPrompter.prompt(&context)?;

    Ok(result.as_bool().unwrap_or(false))
}
