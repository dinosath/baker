use super::Prompter;
use crate::{dialoguer::PromptContext, error::Result};
use dialoguer::{Input, Password};

/// Handles text input prompts including password fields
pub struct TextPrompter;

impl TextPrompter {
    /// Convert serde_json::Value to a default string for input prompts
    fn value_to_default_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => String::new(),
            _ => value.to_string(),
        }
    }
}

impl Prompter<'_> for TextPrompter {
    fn prompt(&self, prompt_context: &PromptContext) -> Result<serde_json::Value> {
        let default_str = self.value_to_default_string(prompt_context.default);

        let input = if let Some(secret) = &prompt_context.question.secret {
            self.prompt_password(prompt_context.help, secret)?
        } else {
            self.prompt_regular_text(prompt_context.help, &default_str)?
        };

        Ok(serde_json::Value::String(input))
    }
}

impl TextPrompter {
    /// Handle password input with optional confirmation
    fn prompt_password(
        &self,
        prompt: &str,
        secret_config: &crate::config::Secret,
    ) -> Result<String> {
        let mut password = Password::new().with_prompt(prompt);

        if secret_config.confirm {
            let error_message = if secret_config.mistmatch_err.is_empty() {
                "Mismatch".to_string()
            } else {
                secret_config.mistmatch_err.clone()
            };

            password =
                password.with_confirmation(format!("{prompt} (confirm)"), error_message);
        }

        Ok(password.interact()?)
    }

    /// Handle regular text input
    fn prompt_regular_text(&self, prompt: &str, default: &str) -> Result<String> {
        Ok(Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text()?)
    }
}
