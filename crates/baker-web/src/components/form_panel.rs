use crate::config::{BakerConfig, FieldType, FormField};
use crate::models::{Loadable, TemplateEntry};
use crate::renderer;
use dioxus::prelude::*;
use indexmap::IndexMap;
use serde_json::Value;

#[component]
pub fn FormPanel(
    selected_template: Signal<Option<TemplateEntry>>,
    config_load: Signal<Loadable<BakerConfig>>,
    mut form_values: Signal<IndexMap<String, Value>>,
    generating: Signal<bool>,
    on_generate: EventHandler<()>,
) -> Element {
    let config = config_load.read().clone();
    let tmpl_name = selected_template.read().as_ref().map(|t| t.name.clone());

    rsx! {
        section { class: "panel panel-form",
            // ── Header ─────────────────────────────────────────────────────────
            div { class: "panel-header",
                h2 { id: "form-title",
                    {tmpl_name.as_deref().unwrap_or("Configure")}
                }
                if let Some(name) = &tmpl_name {
                    span { class: "badge-template", "{name}" }
                }
            }

            // ── Body ───────────────────────────────────────────────────────────
            match config {
                Loadable::Idle => rsx! {
                    div { class: "state-message",
                        EmptyIcon {}
                        p { "Select a template on the left to get started." }
                    }
                },
                Loadable::Loading => rsx! {
                    div { class: "state-message",
                        div { class: "spinner" }
                        span { "Loading template\u{2026}" }
                    }
                },
                Loadable::Failed(msg) => rsx! {
                    div { class: "state-message state-error",
                        ErrorIcon {}
                        p { "{msg}" }
                    }
                },
                Loadable::Ready(cfg) => {
                    let cfg = cfg.clone();
                    rsx! {
                        form {
                            id: "baker-form",
                            onsubmit: move |e| {
                                e.prevent_default();
                                on_generate.call(());
                            },
                            div { class: "form-fields",
                                for field in cfg.fields.iter() {
                                    {render_field(field.clone(), form_values)}
                                }
                            }
                            div { class: "form-actions",
                                button {
                                    class: "btn btn-primary btn-generate",
                                    r#type: "submit",
                                    disabled: *generating.read(),
                                    if *generating.read() {
                                        div { class: "spinner spinner-sm" }
                                        "Generating\u{2026}"
                                    } else {
                                        GenerateIcon {}
                                        "Generate Project"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn should_show(field: &FormField, values: &IndexMap<String, Value>) -> bool {
    if field.ask_if.is_empty() {
        return true;
    }
    let ctx = Value::Object(values.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    match renderer::render(&field.ask_if, &ctx) {
        Ok(result) => {
            let s = result.trim().to_lowercase();
            !matches!(s.as_str(), "false" | "0" | "" | "no" | "none")
        }
        Err(_) => true,
    }
}

fn render_field(
    field: FormField,
    mut form_values: Signal<IndexMap<String, Value>>,
) -> Element {
    let values_snapshot = form_values.read().clone();

    if !should_show(&field, &values_snapshot) {
        return rsx! {};
    }

    let current_value = values_snapshot
        .get(&field.key)
        .cloned()
        .unwrap_or_else(|| field.field_type.initial_value(&field.default));

    let key = field.key.clone();
    let help = field.help.clone();

    match field.field_type {
        // ── Text ───────────────────────────────────────────────────────────────
        FieldType::Text => {
            let val_str = current_value.as_str().unwrap_or_default().to_string();
            let k = key.clone();
            rsx! {
                div { class: "field",
                    label { class: "field-label", r#for: "{key}", "{key}" }
                    if !help.is_empty() {
                        span { class: "field-help", "{help}" }
                    }
                    input {
                        id: "{key}",
                        class: "input",
                        r#type: "text",
                        value: "{val_str}",
                        oninput: move |e| {
                            form_values.write().insert(k.clone(), Value::String(e.value()));
                        },
                    }
                }
            }
        }

        // ── Bool ───────────────────────────────────────────────────────────────
        FieldType::Bool => {
            let checked = current_value.as_bool().unwrap_or(false);
            let k = key.clone();
            rsx! {
                div { class: "field",
                    if !help.is_empty() {
                        label { class: "field-label", "{key}" }
                        span { class: "field-help", "{help}" }
                    }
                    label { class: "toggle-wrap",
                        span { class: "toggle",
                            input {
                                r#type: "checkbox",
                                checked,
                                onchange: move |e| {
                                    form_values
                                        .write()
                                        .insert(k.clone(), Value::Bool(e.checked()));
                                },
                            }
                            span { class: "toggle-track" }
                        }
                        span { class: "toggle-label", "{key}" }
                    }
                }
            }
        }

        // ── Select (single choice) ─────────────────────────────────────────────
        FieldType::Select => {
            let selected_val = current_value.as_str().unwrap_or_default().to_string();
            let choices = field.choices.clone();
            let k = key.clone();
            rsx! {
                div { class: "field",
                    label { class: "field-label", r#for: "{key}", "{key}" }
                    if !help.is_empty() {
                        span { class: "field-help", "{help}" }
                    }
                    select {
                        id: "{key}",
                        class: "input",
                        value: "{selected_val}",
                        onchange: move |e| {
                            form_values
                                .write()
                                .insert(k.clone(), Value::String(e.value()));
                        },
                        for choice in choices.iter() {
                            option {
                                value: "{choice}",
                                selected: *choice == selected_val,
                                "{choice}"
                            }
                        }
                    }
                }
            }
        }

        // ── Multi-Select (chips) ───────────────────────────────────────────────
        FieldType::MultiSelect => {
            let selected_arr: Vec<String> = current_value
                .as_array()
                .map(|a| {
                    a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                })
                .unwrap_or_default();

            let choices = field.choices.clone();
            let k = key.clone();

            rsx! {
                div { class: "field field-full",
                    label { class: "field-label", "{key}" }
                    if !help.is_empty() {
                        span { class: "field-help", "{help}" }
                    }
                    div { class: "chip-group",
                        for choice in choices.iter() {
                            {
                                let choice = choice.clone();
                                let is_sel = selected_arr.contains(&choice);
                                let chip_class = if is_sel { "chip selected" } else { "chip" };
                                let k2 = k.clone();
                                let sel_arr = selected_arr.clone();
                                let c2 = choice.clone();
                                rsx! {
                                    button {
                                        r#type: "button",
                                        class: chip_class,
                                        onclick: move |_| {
                                            let mut next = sel_arr.clone();
                                            if is_sel {
                                                next.retain(|x| x != &c2);
                                            } else {
                                                next.push(c2.clone());
                                            }
                                            let arr = Value::Array(
                                                next.into_iter().map(Value::String).collect(),
                                            );
                                            form_values.write().insert(k2.clone(), arr);
                                        },
                                        "{choice}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── JSON / YAML (textarea) ─────────────────────────────────────────────
        FieldType::Json | FieldType::Yaml => {
            let placeholder = if field.field_type == FieldType::Json {
                "{ \"key\": \"value\" }"
            } else {
                "key: value"
            };
            let val_str = match &current_value {
                Value::String(s) => s.clone(),
                other => serde_json::to_string_pretty(other).unwrap_or_default(),
            };
            let k = key.clone();
            rsx! {
                div { class: "field field-full",
                    label { class: "field-label", r#for: "{key}", "{key}" }
                    if !help.is_empty() {
                        span { class: "field-help", "{help}" }
                    }
                    textarea {
                        id: "{key}",
                        class: "input",
                        placeholder,
                        value: "{val_str}",
                        oninput: move |e| {
                            form_values.write().insert(k.clone(), Value::String(e.value()));
                        },
                    }
                }
            }
        }
    }
}

// ─── Icon helpers ─────────────────────────────────────────────────────────────

#[component]
fn EmptyIcon() -> Element {
    rsx! {
        svg {
            class: "empty-icon",
            view_box: "0 0 64 64",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            rect { x: "12", y: "8", width: "40", height: "48", rx: "4", stroke: "currentColor", stroke_width: "2.5" }
            line { x1: "20", y1: "22", x2: "44", y2: "22", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round" }
            line { x1: "20", y1: "30", x2: "44", y2: "30", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round" }
            line { x1: "20", y1: "38", x2: "34", y2: "38", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round" }
        }
    }
}

#[component]
fn ErrorIcon() -> Element {
    rsx! {
        svg {
            class: "empty-icon",
            view_box: "0 0 64 64",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            circle { cx: "32", cy: "32", r: "26", stroke: "currentColor", stroke_width: "2.5" }
            line { x1: "32", y1: "20", x2: "32", y2: "34", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round" }
            circle { cx: "32", cy: "43", r: "2", fill: "currentColor" }
        }
    }
}

#[component]
fn GenerateIcon() -> Element {
    rsx! {
        svg {
            view_box: "0 0 20 20",
            width: "18",
            height: "18",
            fill: "currentColor",
            path {
                d: "M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z"
            }
        }
    }
}
