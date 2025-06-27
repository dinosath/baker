use super::Prompter;
use crate::{error::Result, prompt::PromptContext};
use dialoguer::{MultiSelect, Select};

/// Handles single choice selection prompts
pub struct SingleChoicePrompter;

impl SingleChoicePrompter {
    /// Find default index for choice selection
    pub fn find_default_choice_index(
        &self,
        choices: &[String],
        default_value: &serde_json::Value,
    ) -> usize {
        match default_value {
            serde_json::Value::String(default_str) => {
                choices.iter().position(|choice| choice == default_str).unwrap_or(0)
            }
            _ => 0,
        }
    }
}

impl Prompter<'_> for SingleChoicePrompter {
    fn prompt(&self, prompt_context: &PromptContext) -> Result<serde_json::Value> {
        let choices = &prompt_context.question.choices;
        let default_index =
            self.find_default_choice_index(choices, prompt_context.default);

        let selection = Select::new()
            .with_prompt(prompt_context.help)
            .default(default_index)
            .items(choices)
            .interact()?;

        Ok(serde_json::Value::String(choices[selection].clone()))
    }
}

/// Handles multiple choice selection prompts
pub struct MultipleChoicePrompter;

impl MultipleChoicePrompter {
    /// Create choice defaults for multiple selection
    fn create_choice_defaults(
        &self,
        choices: &[String],
        default_strings: &[String],
    ) -> Vec<bool> {
        choices.iter().map(|choice| default_strings.contains(choice)).collect()
    }

    /// Extract string array from serde_json::Value
    fn extract_string_array(&self, value: &serde_json::Value) -> Vec<String> {
        match value {
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl Prompter<'_> for MultipleChoicePrompter {
    fn prompt(&self, prompt_context: &PromptContext) -> Result<serde_json::Value> {
        let choices = &prompt_context.question.choices;
        let default_strings = self.extract_string_array(prompt_context.default);
        let defaults = self.create_choice_defaults(choices, &default_strings);

        let indices = MultiSelect::new()
            .with_prompt(prompt_context.help)
            .items(choices)
            .defaults(&defaults)
            .interact()?;

        let selected: Vec<serde_json::Value> = indices
            .iter()
            .map(|&i| serde_json::Value::String(choices[i].clone()))
            .collect();

        Ok(serde_json::Value::Array(selected))
    }
}
