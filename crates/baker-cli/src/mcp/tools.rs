//! MCP tools for Baker.

use crate::store::TemplateStore;
use baker_core::config::{Config, ConfigV1, Type as QuestionTypeEnum};
use rust_mcp_sdk::macros;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Information about a template including its questions and usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Name of the template
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Source (git URL or local path)
    pub source: String,
    /// When the template was installed
    pub installed_at: String,
    /// Questions/variables the template expects
    pub questions: Vec<QuestionInfo>,
    /// Usage example
    pub usage: String,
}

/// Information about a template question/variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInfo {
    /// Variable name
    pub name: String,
    /// Help text describing the variable
    pub help: Option<String>,
    /// Type of the variable (str, bool, json, yaml)
    pub r#type: String,
    /// Default value if any
    pub default: Option<String>,
    /// Whether the variable is required
    pub required: bool,
    /// Available choices for choice-based questions
    pub choices: Option<Vec<String>>,
}

/// Tool for listing all installed templates.
#[macros::mcp_tool(
    name = "list_templates",
    description = "Lists all installed Baker templates with their descriptions, required variables, and usage instructions. Use this to discover available project templates and understand what inputs they need."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ListTemplatesTool {}

impl ListTemplatesTool {
    /// Execute the list templates tool.
    pub fn execute() -> Result<Vec<TemplateInfo>, String> {
        let store = TemplateStore::new().map_err(|e| format!("Failed to access template store: {e}"))?;
        let templates = store.list().map_err(|e| format!("Failed to list templates: {e}"))?;

        let mut result = Vec::new();
        for meta in templates {
            // Try to extract template to read its config
            let questions = match store.extract_to_temp(&meta.name) {
                Ok(temp_dir) => extract_questions(temp_dir.path()),
                Err(_) => Vec::new(),
            };

            let usage = format!(
                "baker generate {} <output_dir> --answers '{{\"variable\": \"value\"}}'",
                meta.name
            );

            result.push(TemplateInfo {
                name: meta.name,
                description: meta.description,
                source: meta.source,
                installed_at: meta.installed_at,
                questions,
                usage,
            });
        }

        Ok(result)
    }
}

/// Tool for generating a project from a template.
#[macros::mcp_tool(
    name = "generate",
    description = "Generates a new project from an installed Baker template. Provide the template name, output directory, and answers to template questions as a JSON object."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct GenerateTool {
    /// Name of the installed template to use
    pub template: String,
    
    /// Output directory for the generated project
    pub output_dir: String,
    
    /// Answers to template questions as a JSON object
    pub answers: HashMap<String, serde_json::Value>,
    
    /// Whether to overwrite existing output directory
    #[serde(default)]
    pub force: bool,
}

impl GenerateTool {
    /// Execute the generate tool.
    pub fn execute(&self) -> Result<String, String> {
        use crate::{run, Args, SkipConfirm};
        use std::path::PathBuf;

        // Verify template exists
        let store = TemplateStore::new().map_err(|e| format!("Failed to access template store: {e}"))?;
        if !store.is_installed(&self.template) {
            return Err(format!("Template '{}' is not installed. Use list_templates to see available templates.", self.template));
        }

        // Convert answers to JSON string
        let answers_json = serde_json::to_string(&self.answers)
            .map_err(|e| format!("Failed to serialize answers: {e}"))?;

        // Create Args for the runner
        let args = Args {
            template: self.template.clone(),
            output_dir: PathBuf::from(&self.output_dir),
            force: self.force,
            verbose: 0,
            answers: Some(answers_json),
            answers_file: None,
            skip_confirms: vec![SkipConfirm::All],
            non_interactive: true,
            dry_run: false,
        };

        // Run the generation
        run(args).map_err(|e| format!("Generation failed: {e}"))?;

        Ok(format!(
            "Successfully generated project from template '{}' in '{}'",
            self.template, self.output_dir
        ))
    }
}

/// Extract question information from a template directory.
fn extract_questions(template_path: &Path) -> Vec<QuestionInfo> {
    let config_path = template_path.join("baker.yaml");
    if !config_path.exists() {
        return Vec::new();
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let config: Config = match serde_yaml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    match config {
        Config::V1(ConfigV1 { questions, .. }) => {
            questions
                .into_iter()
                .map(|(name, q)| {
                    let type_str = match q.r#type {
                        QuestionTypeEnum::Str => "str",
                        QuestionTypeEnum::Bool => "bool",
                        QuestionTypeEnum::Json => "json",
                        QuestionTypeEnum::Yaml => "yaml",
                    };

                    let choices = if q.choices.is_empty() {
                        None
                    } else {
                        Some(q.choices.clone())
                    };

                    let default_str = if q.default.is_null() {
                        None
                    } else if let Some(s) = q.default.as_str() {
                        Some(s.to_string())
                    } else {
                        Some(q.default.to_string())
                    };

                    let help_str = if q.help.is_empty() {
                        None
                    } else {
                        Some(q.help.clone())
                    };

                    QuestionInfo {
                        name,
                        help: help_str,
                        r#type: type_str.to_string(),
                        default: default_str.clone(),
                        required: default_str.is_none() && q.ask_if.is_empty(),
                        choices,
                    }
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates_tool_creation() {
        let tool = ListTemplatesTool {};
        // Just verify the tool can be created
        assert!(std::mem::size_of_val(&tool) >= 0);
    }

    #[test]
    fn test_generate_tool_creation() {
        let tool = GenerateTool {
            template: "test".to_string(),
            output_dir: "/tmp/test".to_string(),
            answers: HashMap::new(),
            force: false,
        };
        assert_eq!(tool.template, "test");
        assert_eq!(tool.output_dir, "/tmp/test");
    }
}
