//! Example tests showing how to mock the new prompt interfaces
//!
//! These tests demonstrate the testability benefits of the new architecture.

#[cfg(test)]
mod tests {
    use super::super::interface::*;
    use crate::error::Result;
    use serde_json::Value;

    /// Mock implementation for testing
    struct MockPrompter {
        text_response: String,
        single_choice_response: usize,
        multiple_choice_response: Vec<usize>,
        confirmation_response: bool,
        structured_data_response: Value,
    }

    impl MockPrompter {
        fn new() -> Self {
            Self {
                text_response: "mock_text".to_string(),
                single_choice_response: 0,
                multiple_choice_response: vec![0, 1],
                confirmation_response: true,
                structured_data_response: Value::Object(Default::default()),
            }
        }

        fn with_text_response(mut self, response: &str) -> Self {
            self.text_response = response.to_string();
            self
        }

        fn with_confirmation_response(mut self, response: bool) -> Self {
            self.confirmation_response = response;
            self
        }
    }

    impl TextPrompter for MockPrompter {
        fn prompt_text(&self, _config: &TextPromptConfig) -> Result<String> {
            Ok(self.text_response.clone())
        }
    }

    impl SingleChoicePrompter for MockPrompter {
        fn prompt_single_choice(&self, _config: &SingleChoiceConfig) -> Result<usize> {
            Ok(self.single_choice_response)
        }
    }

    impl MultipleChoicePrompter for MockPrompter {
        fn prompt_multiple_choice(
            &self,
            _config: &MultipleChoiceConfig,
        ) -> Result<Vec<usize>> {
            Ok(self.multiple_choice_response.clone())
        }
    }

    impl ConfirmationPrompter for MockPrompter {
        fn prompt_confirmation(&self, _config: &ConfirmationConfig) -> Result<bool> {
            Ok(self.confirmation_response)
        }
    }

    impl StructuredDataPrompter for MockPrompter {
        fn prompt_structured_data(
            &self,
            _config: &StructuredDataConfig,
        ) -> Result<Value> {
            Ok(self.structured_data_response.clone())
        }
    }

    #[test]
    fn test_text_prompt_with_mock() {
        let mock = MockPrompter::new().with_text_response("test_input");

        let config = TextPromptConfig {
            prompt: "Enter text:".to_string(),
            default: None,
            secret: None,
        };

        let result = mock.prompt_text(&config).unwrap();
        assert_eq!(result, "test_input");
    }

    #[test]
    fn test_confirmation_prompt_with_mock() {
        let mock = MockPrompter::new().with_confirmation_response(false);

        let config =
            ConfirmationConfig { prompt: "Are you sure?".to_string(), default: true };

        let result = mock.prompt_confirmation(&config).unwrap();
        assert_eq!(result, false);
    }

    #[test]
    fn test_single_choice_prompt_with_mock() {
        let mock = MockPrompter::new();

        let config = SingleChoiceConfig {
            prompt: "Choose:".to_string(),
            choices: vec!["Option 1".to_string(), "Option 2".to_string()],
            default_index: None,
        };

        let result = mock.prompt_single_choice(&config).unwrap();
        assert_eq!(result, 0);
    }
}
