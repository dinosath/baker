use super::Prompter;
use crate::{config::QuestionType, error::Result, prompt::PromptContext};
use dialoguer::{Editor, Input, Select};

/// Handles structured data input (JSON/YAML) with multiple input methods
pub struct StructuredDataPrompter {
    question_type: QuestionType,
}

impl StructuredDataPrompter {
    pub fn new(question_type: QuestionType) -> Self {
        Self { question_type }
    }

    fn is_yaml(&self) -> bool {
        matches!(self.question_type, QuestionType::Yaml)
    }

    fn file_extension(&self) -> &'static str {
        if self.is_yaml() {
            ".yaml"
        } else {
            ".json"
        }
    }

    /// Serialize structured data to string
    fn serialize_structured_data(
        &self,
        value: &serde_json::Value,
        is_yaml: bool,
    ) -> Result<String> {
        if value.is_null() {
            return Ok("{}".to_string());
        }

        if is_yaml {
            Ok(serde_yaml::to_string(value)?)
        } else {
            Ok(serde_json::to_string_pretty(value)?)
        }
    }

    /// Parse structured data content
    fn parse_structured_content(
        &self,
        content: &str,
        is_yaml: bool,
    ) -> Result<serde_json::Value> {
        if content.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }

        if is_yaml {
            Ok(serde_yaml::from_str(content)?)
        } else {
            Ok(serde_json::from_str(content)?)
        }
    }
}

impl Prompter<'_> for StructuredDataPrompter {
    fn prompt(&self, prompt_context: &PromptContext) -> Result<serde_json::Value> {
        let input_method = self.prompt_input_method(prompt_context.help)?;

        match input_method {
            InputMethod::Editor => self.edit_with_external_editor(prompt_context.default),
            InputMethod::Console => self.get_data_from_console(prompt_context.help),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InputMethod {
    Editor = 0,
    Console = 1,
}

impl From<usize> for InputMethod {
    fn from(value: usize) -> Self {
        match value {
            0 => InputMethod::Editor,
            1 => InputMethod::Console,
            _ => InputMethod::Editor,
        }
    }
}

impl StructuredDataPrompter {
    /// Ask user to choose input method
    fn prompt_input_method(&self, prompt: &str) -> Result<InputMethod> {
        let methods = vec!["Use text editor", "Enter inline"];

        let selection = Select::new()
            .with_prompt(format!("{prompt} - Choose input method"))
            .default(0)
            .items(&methods)
            .interact()?;

        Ok(InputMethod::from(selection))
    }

    /// Handle multiline console input for structured data
    fn get_data_from_console(&self, prompt: &str) -> Result<serde_json::Value> {
        println!("{prompt} (Enter empty line to finish):");

        let mut lines = Vec::new();
        loop {
            let line: String =
                Input::new().with_prompt(">").allow_empty(true).interact_text()?;

            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        let content = lines.join("\n");
        self.parse_structured_content(&content, self.is_yaml())
    }

    /// Edit structured data using an external editor
    fn edit_with_external_editor(
        &self,
        default_value: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let default_str =
            self.serialize_structured_data(default_value, self.is_yaml())?;

        if let Some(editor_result) =
            Editor::new().extension(self.file_extension()).edit(&default_str)?
        {
            if editor_result.trim().is_empty() {
                Ok(default_value.clone())
            } else {
                self.parse_structured_content(&editor_result, self.is_yaml())
            }
        } else {
            // User canceled editing
            Ok(default_value.clone())
        }
    }
}
