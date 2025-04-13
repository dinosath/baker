use crate::{
    config::{IntoQuestionType, Question, QuestionType},
    error::Result,
    renderer::TemplateRenderer,
};

#[derive(Debug)]
pub enum ValidationError {
    JsonSchema(String),
    FieldValidation(String),
}

/// Validate a value against a JSON schema.
pub fn validate_with_schema(value: &serde_json::Value, schema: &str) -> Result<()> {
    let schema_value: serde_json::Value = serde_json::from_str(schema)?;

    let validator = jsonschema::validator_for(&schema_value).map_err(|e| {
        crate::error::Error::Other(anyhow::anyhow!("Invalid JSON schema: {}", e))
    })?;

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("Error: {} (at {})", error, error.instance_path))
        .collect();

    if !errors.is_empty() {
        return Err(crate::error::Error::Other(anyhow::anyhow!(
            "Validation failed: {}",
            errors.join("\n")
        )));
    }

    Ok(())
}

pub fn validate_answer(
    question: &Question,
    answer: &serde_json::Value,
    engine: &dyn TemplateRenderer,
    answers: &serde_json::Value,
) -> Result<(), ValidationError> {
    match question.into_question_type() {
        QuestionType::Json | QuestionType::Yaml => {
            if let Some(schema) = &question.schema {
                validate_with_schema(answer, schema).map_err(|e| {
                    ValidationError::JsonSchema(format!(
                        "JSON Schema validation error: {}",
                        e
                    ))
                })?;
            }
        }
        _ => {
            let is_valid = engine
                .execute_expression(&question.validation.condition, answers)
                .unwrap_or(true);

            if !is_valid {
                let error_message = engine
                    .render(&question.validation.error_message, answers)
                    .unwrap_or_else(|_| "Validation failed".to_string());
                return Err(ValidationError::FieldValidation(error_message));
            }
        }
    }
    Ok(())
}
