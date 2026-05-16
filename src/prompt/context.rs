use crate::config::Question;

/// Immutable context passed to prompt providers.
///
/// It bundles the configuration question, resolved default value, and
/// the help text that should be rendered for the user.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Type;

    #[test]
    fn test_prompt_context_new() {
        let question = Question {
            help: "Test help".to_string(),
            r#type: Type::Str,
            default: serde_json::Value::Null,
            choices: vec![],
            multiselect: false,
            secret: None,
            ask_if: "true".to_string(),
            schema: None,
            schema_file: None,
            validation: crate::config::types::get_default_validation(),
        };
        let default = serde_json::Value::String("default_value".to_string());
        let help = "This is a help message";

        let ctx = PromptContext::new(&question, &default, help);

        assert_eq!(ctx.help, help);
        assert_eq!(ctx.default, &default);
    }
}
