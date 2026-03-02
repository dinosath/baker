//! Baker YAML/JSON configuration parser (no native OS deps – WASM-safe)

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

// ─── Wire types (deserialised from baker.yaml) ────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawType {
    Str,
    Bool,
    Json,
    Yaml,
}

impl Default for RawType {
    fn default() -> Self {
        RawType::Str
    }
}

#[derive(Debug, Deserialize)]
struct RawQuestion {
    #[serde(default)]
    help: String,
    #[serde(rename = "type", default)]
    question_type: RawType,
    #[serde(default)]
    default: serde_json::Value,
    #[serde(default)]
    choices: Vec<String>,
    #[serde(default)]
    multiselect: bool,
    #[serde(default)]
    ask_if: String,
}

#[derive(Debug, Deserialize)]
struct RawConfigV1 {
    #[serde(default = "default_suffix")]
    pub template_suffix: String,
    #[serde(default)]
    pub questions: IndexMap<String, RawQuestion>,
}

fn default_suffix() -> String {
    ".baker.j2".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(tag = "schemaVersion")]
enum RawConfig {
    #[serde(rename = "v1")]
    V1(RawConfigV1),
}

// ─── Public domain types ──────────────────────────────────────────────────────

/// The form-field descriptor used to build the UI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    pub key: String,
    pub help: String,
    pub field_type: FieldType,
    pub default: serde_json::Value,
    /// For Select / MultiSelect
    pub choices: Vec<String>,
    /// Jinja2 condition that must evaluate to `true` before the field is shown.
    /// Empty string = always shown.
    pub ask_if: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Bool,
    Select,
    MultiSelect,
    Json,
    Yaml,
}

impl FieldType {
    pub fn initial_value(&self, default: &serde_json::Value) -> serde_json::Value {
        match self {
            FieldType::Bool => {
                serde_json::Value::Bool(default.as_bool().unwrap_or(false))
            }
            FieldType::MultiSelect => {
                if default.is_array() {
                    default.clone()
                } else {
                    serde_json::Value::Array(vec![])
                }
            }
            _ => default.clone(),
        }
    }
}

/// Parsed baker configuration ready to drive the form UI
#[derive(Debug, Clone, PartialEq)]
pub struct BakerConfig {
    pub template_suffix: String,
    pub fields: Vec<FormField>,
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

pub fn parse_config(src: &str) -> Result<BakerConfig, String> {
    // Try YAML first, then JSON
    let raw: RawConfig = if src.trim_start().starts_with('{') {
        serde_json::from_str(src).map_err(|e| format!("JSON parse error: {e}"))?
    } else {
        serde_yaml::from_str(src).map_err(|e| format!("YAML parse error: {e}"))?
    };

    let RawConfig::V1(v1) = raw;

    let fields = v1
        .questions
        .into_iter()
        .map(|(key, q)| {
            let field_type = match (&q.question_type, q.choices.is_empty()) {
                (RawType::Str, false) => {
                    if q.multiselect {
                        FieldType::MultiSelect
                    } else {
                        FieldType::Select
                    }
                }
                (RawType::Str, true) => FieldType::Text,
                (RawType::Bool, _) => FieldType::Bool,
                (RawType::Json, _) => FieldType::Json,
                (RawType::Yaml, _) => FieldType::Yaml,
            };
            FormField {
                key,
                help: q.help,
                field_type,
                default: q.default,
                choices: q.choices,
                ask_if: q.ask_if,
            }
        })
        .collect();

    Ok(BakerConfig { template_suffix: v1.template_suffix, fields })
}
