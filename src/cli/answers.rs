use crate::{
    config::{ConfigV1, QuestionRendered},
    error::{Error, Result},
    prompt::ask_question,
    renderer::TemplateRenderer,
    validation::{validate_answer, ValidationError},
};
use serde_json::{json, Map, Value};

/// Collects answers from various sources: pre-hook output, command line arguments, and user prompts
pub struct AnswerCollector<'a> {
    engine: &'a dyn TemplateRenderer,
    non_interactive: bool,
}

impl<'a> AnswerCollector<'a> {
    pub fn new(engine: &'a dyn TemplateRenderer, non_interactive: bool) -> Self {
        Self { engine, non_interactive }
    }

    /// Read content from a reader into a string.
    fn read_from(&self, mut reader: impl std::io::Read) -> Result<String> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Ok(buf)
    }

    /// Collects answers from all available sources
    pub fn collect_answers(
        &self,
        config: &ConfigV1,
        pre_hook_output: Option<String>,
        cli_answers: Option<String>,
    ) -> Result<Value> {
        let mut answers = Map::new();

        // Add answers from pre-hook output
        if let Some(result) = pre_hook_output {
            log::debug!(
                "Pre-hook stdout content (attempting to parse as JSON answers): {result}"
            );

            let pre_answers = serde_json::from_str::<Value>(&result).map_or_else(
                |e| {
                    log::warn!("Failed to parse hook output as JSON: {e}");
                    Map::new()
                },
                |value| match value {
                    Value::Object(map) => map,
                    _ => Map::new(),
                },
            );
            answers.extend(pre_answers);
        }

        // Add answers from command line arguments
        if let Some(answers_arg) = cli_answers {
            let answers_str = if answers_arg == "-" {
                self.read_from(std::io::stdin())?
            } else {
                answers_arg
            };
            let cli_answers = self.parse_string_to_json(answers_str)?;
            answers.extend(cli_answers);
        }

        // Collect answers for each question through interactive prompts
        for (key, question) in &config.questions {
            self.collect_question_answer(&mut answers, key, question)?;
        }

        Ok(Value::Object(answers))
    }

    /// Collects answer for a single question
    fn collect_question_answer(
        &self,
        answers: &mut Map<String, Value>,
        key: &str,
        question: &crate::config::Question,
    ) -> Result<()> {
        loop {
            let QuestionRendered { help, default, ask_if, .. } =
                question.render(key, &json!(answers), self.engine);

            // Determine if we should skip interactive prompting based on:
            // 1. User explicitly requested non-interactive mode with --non-interactive flag, OR
            // 2. The template's ask_if condition evaluated to false for this question
            let skip_user_prompt = self.non_interactive || !ask_if;

            if skip_user_prompt {
                // Skip to the next question if an answer for this key is already provided
                if answers.contains_key(key) {
                    break;
                }

                // Use the template's default value if one was specified
                if !question.default.is_null() {
                    answers.insert(key.to_string(), default.clone());
                    break;
                }
            }

            let answer = match ask_question(question, &default, help) {
                Ok(answer) => answer,
                Err(err) => match err {
                    Error::JSONParseError(_) | Error::YAMLParseError(_) => {
                        println!("{err}");
                        continue;
                    }
                    _ => return Err(err),
                },
            };

            answers.insert(key.to_string(), answer.clone());
            let _answers = Value::Object(answers.clone());

            match validate_answer(question, &answer, self.engine, &_answers) {
                Ok(_) => break,
                Err(err) => match err {
                    ValidationError::JsonSchema(msg) => println!("{msg}"),
                    ValidationError::FieldValidation(msg) => println!("{msg}"),
                },
            }
        }

        Ok(())
    }

    /// Parse a string into a JSON object.
    pub fn parse_string_to_json(
        &self,
        buf: String,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        let value = serde_json::from_str(&buf)?;

        match value {
            serde_json::Value::Object(map) => Ok(map),
            _ => Ok(serde_json::Map::new()),
        }
    }
}
