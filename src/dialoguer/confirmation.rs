use super::Prompter;
use crate::{dialoguer::PromptContext, error::Result};
use dialoguer::Confirm;

/// Handles boolean confirmation prompts
pub struct ConfirmationPrompter;

impl Prompter<'_> for ConfirmationPrompter {
    fn prompt(&self, prompt_context: &PromptContext) -> Result<serde_json::Value> {
        let default_value = prompt_context.default.as_bool().unwrap_or(false);

        let result = Confirm::new()
            .with_prompt(prompt_context.help)
            .default(default_value)
            .interact()?;

        Ok(serde_json::Value::Bool(result))
    }
}

/// Simple confirmation function for backward compatibility
pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    if skip {
        return Ok(true);
    }

    Ok(Confirm::new().with_prompt(prompt).default(false).interact()?)
}
