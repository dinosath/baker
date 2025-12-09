//! Question configuration and rendering logic

use crate::config::types::{
    get_default_validation, QuestionType, Secret, Type, Validation,
};
use crate::renderer::TemplateRenderer;
use serde::Deserialize;

/// Represents a single question in the configuration
#[derive(Debug, Deserialize)]
pub struct Question {
    /// Help text/prompt to display to the user
    #[serde(default)]
    pub help: String,
    /// Type of the question (string or boolean)
    #[serde(rename = "type")]
    pub r#type: Type,
    /// Optional default value for the question
    #[serde(default)]
    pub default: serde_json::Value,
    /// Available choices for string questions
    #[serde(default)]
    pub choices: Vec<String>,
    /// Available option for string questions
    #[serde(default)]
    pub multiselect: bool,
    /// Whether the string is a secret
    #[serde(default)]
    pub secret: Option<Secret>,
    #[serde(default)]
    pub ask_if: String,
    /// JSON Schema for validation (for Json and Yaml types)
    #[serde(default)]
    pub schema: Option<String>,
    /// Path to a file containing JSON Schema for validation (for Json and Yaml types)
    #[serde(default)]
    pub schema_file: Option<String>,
    #[serde(default = "get_default_validation")]
    pub validation: Validation,
}

#[derive(Debug)]
pub struct QuestionRendered {
    pub ask_if: bool,
    pub default: serde_json::Value,
    pub help: String,
    pub r#type: QuestionType,
}

pub trait IntoQuestionType {
    #[allow(clippy::wrong_self_convention)]
    fn into_question_type(&self) -> QuestionType;
}

impl IntoQuestionType for Question {
    fn into_question_type(&self) -> QuestionType {
        match (&self.r#type, self.choices.is_empty()) {
            (Type::Str, false) => {
                if self.multiselect {
                    QuestionType::MultipleChoice
                } else {
                    QuestionType::SingleChoice
                }
            }
            (Type::Str, true) => QuestionType::Text,
            (Type::Bool, _) => QuestionType::Boolean,
            (Type::Json, _) => QuestionType::Json,
            (Type::Yaml, _) => QuestionType::Yaml,
        }
    }
}

impl Question {
    fn question_type(&self) -> QuestionType {
        self.into_question_type()
    }

    fn prefilled_answer(
        &self,
        question_key: &str,
        answers: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        answers.get(question_key).cloned()
    }

    fn render_default_value(
        &self,
        question_key: &str,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
        question_type: &QuestionType,
    ) -> serde_json::Value {
        if let Some(answer) = self.prefilled_answer(question_key, answers) {
            return answer;
        }

        let default = self.default.clone();
        match question_type {
            QuestionType::MultipleChoice => default,
            QuestionType::Boolean => {
                serde_json::Value::Bool(default.as_bool().unwrap_or(false))
            }
            QuestionType::SingleChoice | QuestionType::Text => {
                self.render_textual_default(default, answers, engine)
            }
            QuestionType::Json | QuestionType::Yaml => {
                self.render_structured_default(default, answers, engine, question_type)
            }
        }
    }

    fn render_textual_default(
        &self,
        default: serde_json::Value,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> serde_json::Value {
        let default_str = default.as_str().unwrap_or_default();
        let rendered = engine
            .render(default_str, answers, Some("default_value"))
            .unwrap_or_default();
        serde_json::Value::String(rendered)
    }

    fn render_structured_default(
        &self,
        default: serde_json::Value,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
        question_type: &QuestionType,
    ) -> serde_json::Value {
        if default.is_object() || default.is_array() {
            return default;
        }

        if let Some(default_str) = default.as_str() {
            let rendered_str = engine
                .render(default_str, answers, Some("default_value"))
                .unwrap_or_else(|_| default_str.to_string());

            return match question_type {
                QuestionType::Json => serde_json::from_str(&rendered_str)
                    .unwrap_or_else(|_| serde_json::json!({})),
                QuestionType::Yaml => serde_yaml::from_str(&rendered_str)
                    .unwrap_or_else(|_| serde_json::json!({})),
                _ => serde_json::json!({}),
            };
        }

        serde_json::json!({})
    }

    fn render_help_text(
        &self,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> String {
        engine.render(&self.help, answers, Some("help")).unwrap_or(self.help.clone())
    }

    fn evaluate_condition(
        &self,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> bool {
        engine.execute_expression(&self.ask_if, answers).unwrap_or(true)
    }

    pub fn render(
        &self,
        question_key: &str,
        answers: &serde_json::Value,
        engine: &dyn TemplateRenderer,
    ) -> QuestionRendered {
        let question_type = self.question_type();
        let default =
            self.render_default_value(question_key, answers, engine, &question_type);
        let help = self.render_help_text(answers, engine);
        let ask_if = self.evaluate_condition(answers, engine);

        QuestionRendered { default, ask_if, help, r#type: question_type }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::MiniJinjaRenderer;
    use serde_json::json;

    fn build_renderer() -> MiniJinjaRenderer {
        MiniJinjaRenderer::new()
    }

    fn base_question(r#type: Type, default: serde_json::Value) -> Question {
        Question {
            help: "Help".to_string(),
            r#type,
            default,
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: String::new(),
            schema: None,
            schema_file: None,
            validation: get_default_validation(),
        }
    }

    #[test]
    fn uses_prefilled_answer_when_present() {
        let question = base_question(Type::Str, json!("ignored"));
        let answers = json!({ "name": "prefilled" });
        let renderer = build_renderer();

        let rendered = question.render("name", &answers, &renderer);

        assert_eq!(rendered.default, json!("prefilled"));
        assert!(rendered.ask_if);
    }

    #[test]
    fn renders_text_default_through_template_engine() {
        let mut question = base_question(Type::Str, json!("{{ project }}"));
        question.help = "{{ project }} help".to_string();
        let answers = json!({ "project": "Demo" });
        let renderer = build_renderer();

        let rendered = question.render("name", &answers, &renderer);

        assert_eq!(rendered.default, json!("Demo"));
        assert_eq!(rendered.help, "Demo help");
    }

    #[test]
    fn renders_structured_json_default_from_string() {
        let question = base_question(Type::Json, json!("{\"enabled\": true}"));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!({ "enabled": true }));
    }

    #[test]
    fn invalid_structured_default_falls_back_to_empty_object() {
        let question = base_question(Type::Json, json!("not-valid"));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!({}));
    }

    #[test]
    fn renders_structured_yaml_default_from_string() {
        let question = base_question(Type::Yaml, json!("enabled: true"));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!({ "enabled": true }));
    }

    #[test]
    fn boolean_defaults_are_rendered_from_value() {
        let question = base_question(Type::Bool, json!(true));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("confirm", &answers, &renderer);

        assert!(rendered.default.as_bool().unwrap());
    }

    #[test]
    fn boolean_defaults_with_null_falls_back_to_false() {
        let question = base_question(Type::Bool, json!(null));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("confirm", &answers, &renderer);

        assert!(!rendered.default.as_bool().unwrap());
    }

    #[test]
    fn structured_default_already_object_returns_as_is() {
        let question = base_question(Type::Json, json!({"key": "value"}));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!({"key": "value"}));
    }

    #[test]
    fn structured_default_already_array_returns_as_is() {
        let question = base_question(Type::Json, json!([1, 2, 3]));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!([1, 2, 3]));
    }

    #[test]
    fn yaml_string_default_parses_as_yaml() {
        // Note: serde_yaml treats simple strings as valid YAML scalars
        // So we test that a valid YAML string parses correctly
        let question = base_question(Type::Yaml, json!("key: value\nlist:\n  - item1"));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!({"key": "value", "list": ["item1"]}));
    }

    #[test]
    fn structured_default_with_number_falls_back_to_empty_object() {
        let question = base_question(Type::Json, json!(42));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("settings", &answers, &renderer);

        assert_eq!(rendered.default, json!({}));
    }

    #[test]
    fn evaluate_condition_with_empty_ask_if_returns_true() {
        let question = base_question(Type::Str, json!("default"));
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("name", &answers, &renderer);

        assert!(rendered.ask_if);
    }

    #[test]
    fn evaluate_condition_with_false_expression() {
        let mut question = base_question(Type::Str, json!("default"));
        question.ask_if = "false".to_string();
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("name", &answers, &renderer);

        assert!(!rendered.ask_if);
    }

    #[test]
    fn evaluate_condition_based_on_answers() {
        let mut question = base_question(Type::Str, json!("default"));
        question.ask_if = "use_db".to_string();
        let answers = json!({"use_db": true});
        let renderer = build_renderer();

        let rendered = question.render("db_name", &answers, &renderer);

        assert!(rendered.ask_if);
    }

    #[test]
    fn question_rendered_debug_impl() {
        let rendered = QuestionRendered {
            ask_if: true,
            default: json!("test"),
            help: "help text".to_string(),
            r#type: QuestionType::Text,
        };
        let debug_str = format!("{:?}", rendered);
        assert!(debug_str.contains("QuestionRendered"));
        assert!(debug_str.contains("ask_if"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn into_question_type_multiple_choice() {
        let mut question = base_question(Type::Str, json!([]));
        question.choices = vec!["a".to_string(), "b".to_string()];
        question.multiselect = true;

        assert_eq!(question.into_question_type(), QuestionType::MultipleChoice);
    }

    #[test]
    fn into_question_type_single_choice() {
        let mut question = base_question(Type::Str, json!(""));
        question.choices = vec!["a".to_string(), "b".to_string()];
        question.multiselect = false;

        assert_eq!(question.into_question_type(), QuestionType::SingleChoice);
    }

    #[test]
    fn into_question_type_text() {
        let question = base_question(Type::Str, json!(""));
        assert_eq!(question.into_question_type(), QuestionType::Text);
    }

    #[test]
    fn into_question_type_boolean() {
        let question = base_question(Type::Bool, json!(false));
        assert_eq!(question.into_question_type(), QuestionType::Boolean);
    }

    #[test]
    fn into_question_type_json() {
        let question = base_question(Type::Json, json!({}));
        assert_eq!(question.into_question_type(), QuestionType::Json);
    }

    #[test]
    fn into_question_type_yaml() {
        let question = base_question(Type::Yaml, json!({}));
        assert_eq!(question.into_question_type(), QuestionType::Yaml);
    }

    #[test]
    fn render_help_text_with_template() {
        let mut question = base_question(Type::Str, json!(""));
        question.help = "Enter name for {{ project }}".to_string();
        let answers = json!({"project": "MyProject"});
        let renderer = build_renderer();

        let rendered = question.render("name", &answers, &renderer);

        assert_eq!(rendered.help, "Enter name for MyProject");
    }

    #[test]
    fn render_help_text_invalid_template_falls_back() {
        let mut question = base_question(Type::Str, json!(""));
        question.help = "Help text with {{ invalid".to_string();
        let answers = json!({});
        let renderer = build_renderer();

        let rendered = question.render("name", &answers, &renderer);

        // Falls back to original help text on error
        assert_eq!(rendered.help, "Help text with {{ invalid");
    }
}
