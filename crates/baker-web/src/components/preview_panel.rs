use dioxus::prelude::*;
use indexmap::IndexMap;

#[component]
pub fn PreviewPanel(
    output_files: Signal<Option<IndexMap<String, String>>>,
    mut selected_file: Signal<Option<String>>,
    on_download: EventHandler<()>,
) -> Element {
    let files = output_files.read().clone();
    let has_output = files.is_some();

    rsx! {
        section { class: "panel panel-preview",
            // ── Header ─────────────────────────────────────────────────────────
            div { class: "panel-header",
                h2 { "Preview" }
                if has_output {
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: move |_| on_download.call(()),
                        DownloadIcon {}
                        " Download ZIP"
                    }
                }
            }

            // ── Body ───────────────────────────────────────────────────────────
            if let Some(ref output) = files {
                div { class: "preview-content",
                    // File tree
                    div { class: "file-tree",
                        for path in output.keys() {
                            {
                                let path = path.clone();
                                let is_active = selected_file
                                    .read()
                                    .as_deref()
                                    .map(|s| s == path)
                                    .unwrap_or(false);
                                let klass = if is_active { "tree-node active" } else { "tree-node" };
                                let p2 = path.clone();
                                rsx! {
                                    div {
                                        class: klass,
                                        onclick: move |_| selected_file.set(Some(p2.clone())),
                                        FileIcon { path: path.clone() }
                                        span { "{path}" }
                                    }
                                }
                            }
                        }
                    }

                    // File viewer
                    {
                        let sel = selected_file.read().clone();
                        let content = sel
                            .as_deref()
                            .and_then(|p| output.get(p))
                            .cloned()
                            .unwrap_or_default();
                        let filename = sel.as_deref().unwrap_or("—");
                        rsx! {
                            div { class: "file-viewer",
                                div { class: "file-viewer-header", "{filename}" }
                                pre { class: "file-viewer-code", "{content}" }
                            }
                        }
                    }
                }
            } else {
                div { class: "state-message",
                    FolderIcon {}
                    p { "Generate the project to see a file preview." }
                }
            }
        }
    }
}

// ─── Icon components ──────────────────────────────────────────────────────────

#[component]
fn DownloadIcon() -> Element {
    rsx! {
        svg {
            view_box: "0 0 20 20",
            width: "16",
            height: "16",
            fill: "currentColor",
            path {
                fill_rule: "evenodd",
                clip_rule: "evenodd",
                d: "M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm3.293-7.707a1 1 0 011.414 0L9 10.586V3a1 1 0 112 0v7.586l1.293-1.293a1 1 0 111.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z"
            }
        }
    }
}

#[component]
fn FolderIcon() -> Element {
    rsx! {
        svg {
            class: "empty-icon",
            view_box: "0 0 64 64",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            path {
                d: "M8 52V20l16-8 16 8 16-8v32l-16 8-16-8-16 8z",
                stroke: "currentColor",
                stroke_width: "2.5",
                stroke_linejoin: "round"
            }
        }
    }
}

/// Pick a simple icon based on the file extension
#[component]
fn FileIcon(path: String) -> Element {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    let icon = match ext.as_str() {
        "rs" => "🦀",
        "py" => "🐍",
        "js" | "ts" => "📜",
        "json" => "{}",
        "yaml" | "yml" => "⚙",
        "md" => "📝",
        "toml" => "⚙",
        "html" | "htm" => "🌐",
        "sh" | "bash" => "💲",
        "dockerfile" | "containerfile" => "🐳",
        _ => "📄",
    };
    rsx! { span { class: "tree-icon", "{icon}" } }
}
