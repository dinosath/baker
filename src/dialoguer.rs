use crate::{
    config::Question,
    error::{Error, Result},
};

use dialoguer::{Confirm, Input, MultiSelect, Password, Select};

pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    if skip {
        return Ok(true);
    }
    Confirm::new().with_prompt(prompt).default(false).interact().map_err(Error::IoError)
}

pub fn prompt_multiple_choice(
    choices: Vec<String>,
    default_value: serde_json::Value,
    prompt: String,
) -> Result<serde_json::Value> {
    let default_strings: Vec<String> = match default_value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let defaults: Vec<bool> =
        choices.iter().map(|choice| default_strings.contains(choice)).collect();

    let indices = MultiSelect::new()
        .with_prompt(prompt)
        .items(&choices)
        .defaults(&defaults)
        .interact()
        .map_err(Error::IoError)?;

    let selected: Vec<serde_json::Value> =
        indices.iter().map(|&i| serde_json::Value::String(choices[i].clone())).collect();

    Ok(serde_json::Value::Array(selected))
}

pub fn prompt_boolean(
    default_value: serde_json::Value,
    prompt: String,
) -> Result<serde_json::Value> {
    let default_value = default_value.as_bool().unwrap();
    let result = Confirm::new()
        .with_prompt(prompt)
        .default(default_value)
        .interact()
        .map_err(Error::IoError)?;

    Ok(serde_json::Value::Bool(result))
}

pub fn prompt_single_choice(
    choices: Vec<String>,
    default_value: serde_json::Value,
    prompt: String,
) -> Result<serde_json::Value> {
    let default_value: usize = match &default_value {
        serde_json::Value::String(default_str) => {
            choices.iter().position(|choice| choice == default_str).unwrap_or(0)
        }
        _ => 0,
    };
    let selection = Select::new()
        .with_prompt(prompt)
        .default(default_value)
        .items(&choices)
        .interact()
        .map_err(Error::IoError)?;

    Ok(serde_json::Value::String(choices[selection].clone()))
}

pub fn prompt_text(
    question: &Question,
    default_value: serde_json::Value,
    prompt: String,
) -> Result<serde_json::Value> {
    let default_str = match default_value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        _ => default_value.to_string(),
    };

    let input = if let Some(secret) = &question.secret {
        let mut password = Password::new();
        let mut password = password.with_prompt(&prompt);

        if secret.confirm {
            password = password.with_confirmation(
                format!("{} (confirm)", &prompt),
                if secret.mistmatch_err.is_empty() {
                    "Mistmatch".to_string()
                } else {
                    secret.mistmatch_err.clone()
                },
            );
        }

        password.interact().map_err(Error::IoError)?
    } else {
        Input::new()
            .with_prompt(&prompt)
            .default(default_str)
            .interact_text()
            .map_err(Error::IoError)?
    };

    Ok(serde_json::Value::String(input))
}
