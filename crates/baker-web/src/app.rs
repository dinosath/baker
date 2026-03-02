use crate::{
    components::{
        form_panel::FormPanel, preview_panel::PreviewPanel, template_panel::TemplatePanel,
        topbar::Topbar,
    },
    config, github,
    models::{Loadable, Tab, TemplateEntry},
    renderer, zip,
};
use dioxus::prelude::*;
use indexmap::IndexMap;
use serde_json::Value;

#[component]
pub fn App() -> Element {
    // ── Global signals ───────────────────────────────────────────────────────
    let tab: Signal<Tab> = use_signal(|| Tab::Community);
    let search: Signal<String> = use_signal(String::new);
    let custom_url_input: Signal<String> = use_signal(String::new);

    let mut selected_template: Signal<Option<TemplateEntry>> = use_signal(|| None);
    let custom_templates: Signal<Vec<TemplateEntry>> = use_signal(Vec::new);

    let mut config_load: Signal<Loadable<crate::config::BakerConfig>> =
        use_signal(|| Loadable::Idle);
    let mut form_values: Signal<IndexMap<String, Value>> = use_signal(IndexMap::new);

    let mut output_files: Signal<Option<IndexMap<String, String>>> = use_signal(|| None);
    let mut selected_file: Signal<Option<String>> = use_signal(|| None);
    let mut generating: Signal<bool> = use_signal(|| false);

    // Derived project name for ZIP filename
    let mut project_name: Signal<String> = use_signal(|| "project".to_string());

    // ── on_select: user picks a template ────────────────────────────────────
    let on_select = move |entry: TemplateEntry| {
        // Reset state
        config_load.set(Loadable::Loading);
        form_values.set(IndexMap::new());
        output_files.set(None);
        selected_file.set(None);
        project_name.set(entry.slug());
        selected_template.set(Some(entry.clone()));

        spawn(async move {
            match github::fetch_baker_yaml(&entry).await {
                Ok(yaml_str) => match config::parse_config(&yaml_str) {
                    Ok(cfg) => {
                        // Seed form values with field defaults
                        let initial: IndexMap<String, Value> = cfg
                            .fields
                            .iter()
                            .map(|f| {
                                (
                                    f.key.clone(),
                                    f.field_type.initial_value(&f.default),
                                )
                            })
                            .collect();
                        form_values.set(initial);
                        config_load.set(Loadable::Ready(cfg));
                    }
                    Err(e) => {
                        config_load.set(Loadable::Failed(format!("Config parse error: {e}")));
                    }
                },
                Err(e) => {
                    config_load.set(Loadable::Failed(e));
                }
            }
        });
    };

    // ── on_generate: user submits the form ──────────────────────────────────
    let on_generate = move |()| {
        let entry = selected_template.read().clone();
        let cfg = match config_load.read().clone() {
            Loadable::Ready(c) => c,
            _ => return,
        };
        let answers = form_values.read().clone();

        if let Some(entry) = entry {
            generating.set(true);
            output_files.set(None);
            selected_file.set(None);

            spawn(async move {
                // Build rendering context from current form values
                let ctx = Value::Object(
                    answers
                        .into_iter()
                        .map(|(k, v)| (k, v))
                        .collect(),
                );

                match github::fetch_template_files(&entry).await {
                    Ok(files) => {
                        match renderer::render_project(&cfg.template_suffix, &files, &ctx) {
                            Ok(out) => {
                                // Auto-select first file for preview
                                if let Some(first) = out.keys().next().cloned() {
                                    selected_file.set(Some(first));
                                }
                                output_files.set(Some(out));
                            }
                            Err(e) => {
                                log::error!("Render error: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Fetch template files error: {e}");
                    }
                }

                generating.set(false);
            });
        }
    };

    // ── on_download: user clicks Download ZIP ────────────────────────────────
    let on_download = move |()| {
        if let Some(ref files) = *output_files.read() {
            let pname = project_name.read().clone();
            match zip::build_zip(files) {
                Ok(bytes) => {
                    if let Err(e) = zip::trigger_download(bytes, &format!("{pname}.zip")) {
                        log::error!("Download error: {e}");
                    }
                }
                Err(e) => {
                    log::error!("ZIP build error: {e}");
                }
            }
        }
    };

    // ── Render ───────────────────────────────────────────────────────────────
    rsx! {
        Topbar {}

        // Hero banner
        section { class: "hero",
            div { class: "hero-inner",
                h1 { class: "hero-title",
                    "Bootstrap your project"
                    br {}
                    span { class: "hero-accent", "in seconds" }
                }
                p { class: "hero-sub",
                    "Pick a template, fill in the form, download your project — all in-browser, "
                    "powered by Baker running on WebAssembly."
                }
            }
        }

        // Three-panel workspace
        main { class: "workspace",
            TemplatePanel {
                selected: selected_template,
                tab,
                search,
                custom_url_input,
                custom_templates,
                on_select,
            }
            FormPanel {
                selected_template,
                config_load,
                form_values,
                generating,
                on_generate,
            }
            PreviewPanel {
                output_files,
                selected_file,
                on_download,
            }
        }

        // Footer
        footer { class: "footer",
            div { class: "footer-inner",
                span { "Baker © 2024 · MIT License" }
                span { "Running on "  strong { "WebAssembly" }  " · No server involved" }
                a {
                    href: "https://github.com/aliev/baker",
                    target: "_blank",
                    rel: "noopener",
                    "Contribute on GitHub"
                }
            }
        }
    }
}
