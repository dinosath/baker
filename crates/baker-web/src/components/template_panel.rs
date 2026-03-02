use crate::models::{Tab, TemplateEntry};
use crate::templates::{community_templates, parse_github_url};
use dioxus::prelude::*;

#[component]
pub fn TemplatePanel(
    selected: Signal<Option<TemplateEntry>>,
    mut tab: Signal<Tab>,
    mut search: Signal<String>,
    mut custom_url_input: Signal<String>,
    mut custom_templates: Signal<Vec<TemplateEntry>>,
    on_select: EventHandler<TemplateEntry>,
) -> Element {
    let community = community_templates();

    // Filtered community list
    let q = search.read().to_lowercase();
    let filtered: Vec<TemplateEntry> = community
        .into_iter()
        .filter(|t| {
            q.is_empty()
                || t.name.to_lowercase().contains(&q)
                || t.description.to_lowercase().contains(&q)
                || t.tags.iter().any(|tag| tag.to_lowercase().contains(&q))
        })
        .collect();

    let visible_custom = custom_templates.read().clone();
    let active_tab = *tab.read();

    let mut url_error: Signal<Option<String>> = use_signal(|| None);

    rsx! {
        aside { class: "panel panel-templates",
            // ── Header ───────────────────────────────────────────────────────
            div { class: "panel-header",
                h2 { "Templates" }
            }

            // ── Search ───────────────────────────────────────────────────────
            div { class: "template-search-wrap",
                input {
                    class: "input",
                    r#type: "text",
                    placeholder: "Search templates…",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
            }

            // ── Custom repo input ─────────────────────────────────────────────
            div { class: "custom-repo-wrap",
                label { class: "field-label", r#for: "custom-repo", "Custom GitHub URL" }
                div { class: "input-addon",
                    input {
                        id: "custom-repo",
                        class: "input",
                        r#type: "text",
                        placeholder: "https://github.com/owner/repo",
                        value: "{custom_url_input}",
                        oninput: move |e| {
                            custom_url_input.set(e.value());
                            url_error.set(None);
                        },
                        onkeydown: move |e: Event<KeyboardData>| {
                            if e.key() == Key::Enter {
                                load_custom(custom_url_input, custom_templates, tab, url_error);
                            }
                        },
                    }
                    button {
                        class: "btn btn-sm btn-primary",
                        onclick: move |_| {
                            load_custom(custom_url_input, custom_templates, tab, url_error);
                        },
                        "Load"
                    }
                }
                if let Some(err) = url_error.read().as_deref() {
                    p { class: "input-error", "{err}" }
                }
            }

            // ── Tab bar ───────────────────────────────────────────────────────
            div { class: "template-tabs",
                button {
                    class: if active_tab == Tab::Community { "tab active" } else { "tab" },
                    onclick: move |_| tab.set(Tab::Community),
                    "Community"
                }
                button {
                    class: if active_tab == Tab::Custom { "tab active" } else { "tab" },
                    onclick: move |_| tab.set(Tab::Custom),
                    "Custom "
                    span { class: "tab-count",
                        "({visible_custom.len()})"
                    }
                }
            }

            // ── Template list ─────────────────────────────────────────────────
            div { class: "template-list",
                if active_tab == Tab::Community {
                    if filtered.is_empty() {
                        div { class: "list-empty", "No templates match your search." }
                    }
                    for t in filtered.iter() {
                        {
                            let t = t.clone();
                            let is_active = selected
                                .read()
                                .as_ref()
                                .map(|s| s.name == t.name && s.owner == t.owner)
                                .unwrap_or(false);
                            let klass = if is_active {
                                "template-item active"
                            } else {
                                "template-item"
                            };
                            let t2 = t.clone();
                            rsx! {
                                div {
                                    class: klass,
                                    onclick: move |_| on_select.call(t2.clone()),
                                    div { class: "template-item-name", "{t.name}" }
                                    div { class: "template-item-desc", "{t.description}" }
                                    div { class: "template-item-tags",
                                        for tag in t.tags.iter() {
                                            span { class: "tag", "{tag}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if visible_custom.is_empty() {
                        div { class: "list-empty",
                            "No custom templates loaded yet."
                            br {}
                            "Paste a GitHub URL above and click Load."
                        }
                    }
                    for t in visible_custom.iter() {
                        {
                            let t = t.clone();
                            let is_active = selected
                                .read()
                                .as_ref()
                                .map(|s| s.name == t.name && s.owner == t.owner)
                                .unwrap_or(false);
                            let klass = if is_active {
                                "template-item active"
                            } else {
                                "template-item"
                            };
                            let t2 = t.clone();
                            rsx! {
                                div {
                                    class: klass,
                                    onclick: move |_| on_select.call(t2.clone()),
                                    div { class: "template-item-name", "{t.name}" }
                                    div { class: "template-item-desc", "{t.description}" }
                                    div { class: "template-item-tags",
                                        for tag in t.tags.iter() {
                                            span { class: "tag", "{tag}" }
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
}

fn load_custom(
    custom_url_input: Signal<String>,
    mut custom_templates: Signal<Vec<TemplateEntry>>,
    mut tab: Signal<Tab>,
    mut url_error: Signal<Option<String>>,
) {
    let url = custom_url_input.read().clone();
    if url.trim().is_empty() {
        return;
    }
    match parse_github_url(&url) {
        Ok(entry) => {
            custom_templates.write().push(entry);
            tab.set(Tab::Custom);
            url_error.set(None);
        }
        Err(e) => {
            url_error.set(Some(e));
        }
    }
}
