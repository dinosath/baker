//! Basic types and enums for configuration

use crate::constants::validation;
use serde::Deserialize;

/// Type of question to be presented to the user
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Type {
    /// String input question type
    Str,
    /// Boolean (yes/no) question type
    Bool,
    /// JSON structured input type
    Json,
    /// YAML structured input type
    Yaml,
}

#[derive(Debug, Deserialize)]
pub struct Secret {
    /// Whether the secret should have confirmation
    #[serde(default)]
    pub confirm: bool,
    #[serde(default = "get_default_mismatch_error")]
    pub mistmatch_err: String,
}

#[derive(Debug, Deserialize)]
pub struct Validation {
    #[serde(default = "get_default_condition")]
    pub condition: String,
    #[serde(default = "get_default_error_message")]
    pub error_message: String,
}

#[derive(Debug, PartialEq)]
pub enum QuestionType {
    MultipleChoice,
    SingleChoice,
    Text,
    Boolean,
    Json,
    Yaml,
}

fn get_default_error_message() -> String {
    validation::INVALID_ANSWER.to_string()
}

fn get_default_mismatch_error() -> String {
    validation::PASSWORDS_MISMATCH.to_string()
}

pub fn get_default_condition() -> String {
    validation::DEFAULT_CONDITION.to_string()
}

pub fn get_default_validation() -> Validation {
    Validation {
        condition: get_default_condition(),
        error_message: get_default_error_message(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_error_message() {
        let msg = get_default_error_message();
        assert_eq!(msg, crate::constants::validation::INVALID_ANSWER);
    }

    #[test]
    fn test_get_default_mismatch_error() {
        let msg = get_default_mismatch_error();
        assert_eq!(msg, crate::constants::validation::PASSWORDS_MISMATCH);
    }

    #[test]
    fn test_get_default_condition() {
        let condition = get_default_condition();
        assert_eq!(condition, crate::constants::validation::DEFAULT_CONDITION);
    }

    #[test]
    fn test_get_default_validation() {
        let validation = get_default_validation();
        assert_eq!(validation.condition, "true");
        assert_eq!(
            validation.error_message,
            crate::constants::validation::INVALID_ANSWER
        );
    }

    #[test]
    fn test_question_type_equality() {
        assert_eq!(QuestionType::MultipleChoice, QuestionType::MultipleChoice);
        assert_eq!(QuestionType::SingleChoice, QuestionType::SingleChoice);
        assert_eq!(QuestionType::Text, QuestionType::Text);
        assert_eq!(QuestionType::Boolean, QuestionType::Boolean);
        assert_eq!(QuestionType::Json, QuestionType::Json);
        assert_eq!(QuestionType::Yaml, QuestionType::Yaml);

        assert_ne!(QuestionType::Text, QuestionType::Boolean);
        assert_ne!(QuestionType::Json, QuestionType::Yaml);
    }

    #[test]
    fn test_type_enum_deserialize_str() {
        let json = r#""str""#;
        let t: Type = serde_json::from_str(json).unwrap();
        assert!(matches!(t, Type::Str));
    }

    #[test]
    fn test_type_enum_deserialize_bool() {
        let json = r#""bool""#;
        let t: Type = serde_json::from_str(json).unwrap();
        assert!(matches!(t, Type::Bool));
    }

    #[test]
    fn test_type_enum_deserialize_json() {
        let json = r#""json""#;
        let t: Type = serde_json::from_str(json).unwrap();
        assert!(matches!(t, Type::Json));
    }

    #[test]
    fn test_type_enum_deserialize_yaml() {
        let json = r#""yaml""#;
        let t: Type = serde_json::from_str(json).unwrap();
        assert!(matches!(t, Type::Yaml));
    }

    #[test]
    fn test_type_enum_debug() {
        assert!(format!("{:?}", Type::Str).contains("Str"));
        assert!(format!("{:?}", Type::Bool).contains("Bool"));
        assert!(format!("{:?}", Type::Json).contains("Json"));
        assert!(format!("{:?}", Type::Yaml).contains("Yaml"));
    }

    #[test]
    fn test_secret_deserialize() {
        let json = r#"{"confirm": true, "mistmatch_err": "Custom error"}"#;
        let secret: Secret = serde_json::from_str(json).unwrap();
        assert!(secret.confirm);
        assert_eq!(secret.mistmatch_err, "Custom error");
    }

    #[test]
    fn test_secret_deserialize_defaults() {
        let json = r#"{}"#;
        let secret: Secret = serde_json::from_str(json).unwrap();
        assert!(!secret.confirm);
        assert_eq!(
            secret.mistmatch_err,
            crate::constants::validation::PASSWORDS_MISMATCH
        );
    }

    #[test]
    fn test_validation_deserialize() {
        let json = r#"{"condition": "value != ''", "error_message": "Cannot be empty"}"#;
        let validation: Validation = serde_json::from_str(json).unwrap();
        assert_eq!(validation.condition, "value != ''");
        assert_eq!(validation.error_message, "Cannot be empty");
    }

    #[test]
    fn test_validation_deserialize_defaults() {
        let json = r#"{}"#;
        let validation: Validation = serde_json::from_str(json).unwrap();
        assert_eq!(validation.condition, crate::constants::validation::DEFAULT_CONDITION);
        assert_eq!(
            validation.error_message,
            crate::constants::validation::INVALID_ANSWER
        );
    }

    #[test]
    fn test_secret_debug() {
        let secret = Secret { confirm: true, mistmatch_err: "test".to_string() };
        let debug_str = format!("{:?}", secret);
        assert!(debug_str.contains("Secret"));
        assert!(debug_str.contains("confirm"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_validation_debug() {
        let validation = Validation {
            condition: "test_condition".to_string(),
            error_message: "test_message".to_string(),
        };
        let debug_str = format!("{:?}", validation);
        assert!(debug_str.contains("Validation"));
        assert!(debug_str.contains("test_condition"));
        assert!(debug_str.contains("test_message"));
    }
}
