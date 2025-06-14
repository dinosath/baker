use crate::{
    config::{IntoQuestionType, Question, QuestionType},
    error::Result,
};

use dialoguer::{Confirm, Editor, Input, MultiSelect, Password, Select};

pub fn confirm(skip: bool, prompt: String) -> Result<bool> {
    if skip {
        return Ok(true);
    }
    Ok(Confirm::new().with_prompt(prompt).default(false).interact()?)
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
        .interact()?;

    let selected: Vec<serde_json::Value> =
        indices.iter().map(|&i| serde_json::Value::String(choices[i].clone())).collect();

    Ok(serde_json::Value::Array(selected))
}

pub fn prompt_boolean(
    default_value: serde_json::Value,
    prompt: String,
) -> Result<serde_json::Value> {
    let default_value = default_value.as_bool().unwrap();
    let result = Confirm::new().with_prompt(prompt).default(default_value).interact()?;

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
        .interact()?;

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
        let password = Password::new();
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

        password.interact()?
    } else {
        Input::new().with_prompt(&prompt).default(default_str).interact_text()?
    };

    Ok(serde_json::Value::String(input))
}

/// Asks user for input method for structured data
fn prompt_for_input_method(prompt: &str, default_method: usize) -> Result<usize> {
    let methods = vec!["Use text editor", "Enter inline"];

    let selection = Select::new()
        .with_prompt(format!("{} - Choose input method", prompt))
        .default(default_method)
        .items(&methods)
        .interact()?;

    Ok(selection)
}

/// Handle multiline console input for structured data
fn get_data_from_console(is_yaml: bool, prompt: &str) -> Result<serde_json::Value> {
    println!("{} (Enter empty line to finish):", prompt);
    let mut lines = Vec::new();
    loop {
        let line: String =
            Input::new().with_prompt(">").allow_empty(true).interact_text()?;
        if line.is_empty() {
            break;
        }
        lines.push(line);
    }

    let content = lines.join("\n");
    if content.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }

    if is_yaml {
        Ok(serde_yaml::from_str(&content)?)
    } else {
        Ok(serde_json::from_str(&content)?)
    }
}

/// Edit structured data using an external editor
fn edit_with_external_editor(
    default_value: serde_json::Value,
    is_yaml: bool,
    extension: &str,
) -> Result<serde_json::Value> {
    let default_str = if default_value.is_null() {
        "{}".to_string()
    } else if is_yaml {
        serde_yaml::to_string(&default_value)?
    } else {
        serde_json::to_string_pretty(&default_value)?
    };

    if let Some(editor_result) = Editor::new().extension(extension).edit(&default_str)? {
        if editor_result.trim().is_empty() {
            Ok(default_value)
        } else if is_yaml {
            Ok(serde_yaml::from_str(&editor_result)?)
        } else {
            Ok(serde_json::from_str(&editor_result)?)
        }
    } else {
        // User canceled editing
        Ok(default_value)
    }
}

/// Prompt for structured data (JSON or YAML)
pub fn prompt_structured_data(
    default_value: serde_json::Value,
    prompt: String,
    question_type: QuestionType,
) -> Result<serde_json::Value> {
    let is_yaml = matches!(question_type, QuestionType::Yaml);
    let extension = if is_yaml { ".yaml" } else { ".json" };
    let input_method = prompt_for_input_method(&prompt, 0)?;

    let result = match input_method {
        0 => edit_with_external_editor(default_value.clone(), is_yaml, extension)?,
        1 => get_data_from_console(is_yaml, &prompt)?,
        _ => default_value,
    };

    Ok(result)
}

pub fn ask_question(
    question: &Question,
    default: &serde_json::Value,
    help: String,
) -> Result<serde_json::Value> {
    match question.into_question_type() {
        QuestionType::MultipleChoice => prompt_multiple_choice(
            question.choices.clone(),
            default.clone(),
            help.clone(),
        ),
        QuestionType::Boolean => prompt_boolean(default.clone(), help.clone()),
        QuestionType::SingleChoice => {
            prompt_single_choice(question.choices.clone(), default.clone(), help.clone())
        }
        QuestionType::Text => prompt_text(question, default.clone(), help.clone()),
        QuestionType::Json | QuestionType::Yaml => prompt_structured_data(
            default.clone(),
            help.clone(),
            question.into_question_type(),
        ),
    }
}
