use crate::error::Result;
use serde_json::Value;

pub struct DataParser;

impl DataParser {
    /// Parse structured data content
    pub fn parse_structured_content(content: &str, is_yaml: bool) -> Result<Value> {
        if content.trim().is_empty() {
            return Ok(Value::Object(serde_json::Map::new()));
        }

        if is_yaml {
            Ok(serde_yaml::from_str(content)?)
        } else {
            Ok(serde_json::from_str(content)?)
        }
    }

    /// Serialize structured data to string
    pub fn serialize_structured_data(value: &Value, is_yaml: bool) -> Result<String> {
        if value.is_null() {
            return Ok("{}".to_string());
        }

        if is_yaml {
            Ok(serde_yaml::to_string(value)?)
        } else {
            Ok(serde_json::to_string_pretty(value)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_structured_content_empty_string() {
        let result = DataParser::parse_structured_content("", false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Object(serde_json::Map::new()));
    }

    #[test]
    fn test_parse_structured_content_whitespace_only() {
        let result = DataParser::parse_structured_content("   \n\t  ", true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Object(serde_json::Map::new()));
    }

    #[test]
    fn test_parse_structured_content_json_valid() {
        let json_content = r#"{"name": "test", "value": 42}"#;
        let result = DataParser::parse_structured_content(json_content, false);
        assert!(result.is_ok());
        let expected = json!({"name": "test", "value": 42});
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_structured_content_json_array() {
        let json_content = r#"[1, 2, 3, "test"]"#;
        let result = DataParser::parse_structured_content(json_content, false);
        assert!(result.is_ok());
        let expected = json!([1, 2, 3, "test"]);
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_structured_content_json_invalid() {
        let json_content = r#"{"name": "test", "value":}"#; // Invalid JSON
        let result = DataParser::parse_structured_content(json_content, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_structured_content_yaml_valid() {
        let yaml_content = "name: test\nvalue: 42\nlist:\n  - item1\n  - item2";
        let result = DataParser::parse_structured_content(yaml_content, true);
        assert!(result.is_ok());
        let expected = json!({
            "name": "test",
            "value": 42,
            "list": ["item1", "item2"]
        });
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_structured_content_yaml_invalid() {
        let yaml_content = "name: test\n  value: 42"; // Invalid YAML indentation
        let result = DataParser::parse_structured_content(yaml_content, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_structured_content_yaml_simple() {
        let yaml_content = "key: value";
        let result = DataParser::parse_structured_content(yaml_content, true);
        assert!(result.is_ok());
        let expected = json!({"key": "value"});
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_serialize_structured_data_null_json() {
        let result = DataParser::serialize_structured_data(&Value::Null, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "{}");
    }

    #[test]
    fn test_serialize_structured_data_null_yaml() {
        let result = DataParser::serialize_structured_data(&Value::Null, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "{}");
    }

    #[test]
    fn test_serialize_structured_data_json_object() {
        let data = json!({"name": "test", "value": 42});
        let result = DataParser::serialize_structured_data(&data, false);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("\"name\": \"test\""));
        assert!(output.contains("\"value\": 42"));
    }

    #[test]
    fn test_serialize_structured_data_json_array() {
        let data = json!([1, 2, 3]);
        let result = DataParser::serialize_structured_data(&data, false);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("1"));
        assert!(output.contains("2"));
        assert!(output.contains("3"));
    }

    #[test]
    fn test_serialize_structured_data_yaml_object() {
        let data = json!({"name": "test", "value": 42});
        let result = DataParser::serialize_structured_data(&data, true);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("name: test"));
        assert!(output.contains("value: 42"));
    }

    #[test]
    fn test_serialize_structured_data_yaml_array() {
        let data = json!(["item1", "item2"]);
        let result = DataParser::serialize_structured_data(&data, true);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("- item1"));
        assert!(output.contains("- item2"));
    }

    #[test]
    fn test_round_trip_json() {
        let original_json = r#"{"name": "test", "nested": {"value": 42}}"#;
        let parsed = DataParser::parse_structured_content(original_json, false).unwrap();
        let serialized = DataParser::serialize_structured_data(&parsed, false).unwrap();
        let reparsed = DataParser::parse_structured_content(&serialized, false).unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_round_trip_yaml() {
        let original_yaml = "name: test\nnested:\n  value: 42";
        let parsed = DataParser::parse_structured_content(original_yaml, true).unwrap();
        let serialized = DataParser::serialize_structured_data(&parsed, true).unwrap();
        let reparsed = DataParser::parse_structured_content(&serialized, true).unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn test_cross_format_conversion() {
        let json_content = r#"{"name": "test", "value": 42}"#;
        let parsed_from_json =
            DataParser::parse_structured_content(json_content, false).unwrap();
        let yaml_output =
            DataParser::serialize_structured_data(&parsed_from_json, true).unwrap();
        let parsed_from_yaml =
            DataParser::parse_structured_content(&yaml_output, true).unwrap();
        assert_eq!(parsed_from_json, parsed_from_yaml);
    }

    #[test]
    fn test_complex_nested_structure() {
        let complex_data = json!({
            "users": [
                {"id": 1, "name": "Alice", "active": true},
                {"id": 2, "name": "Bob", "active": false}
            ],
            "settings": {
                "theme": "dark",
                "notifications": {
                    "email": true,
                    "push": false
                }
            },
            "version": "1.0.0"
        });

        // Test JSON serialization/parsing
        let json_str =
            DataParser::serialize_structured_data(&complex_data, false).unwrap();
        let parsed_json = DataParser::parse_structured_content(&json_str, false).unwrap();
        assert_eq!(complex_data, parsed_json);

        // Test YAML serialization/parsing
        let yaml_str =
            DataParser::serialize_structured_data(&complex_data, true).unwrap();
        let parsed_yaml = DataParser::parse_structured_content(&yaml_str, true).unwrap();
        assert_eq!(complex_data, parsed_yaml);
    }
}
