# baker-web

**Baker** — In-browser project generator built with [Dioxus](https://dioxuslabs.com/) and
compiled to WebAssembly.  
Inspired by [start.spring.io](https://start.spring.io/), [code.quarkus.io](https://code.quarkus.io/)
and [bootify.io](https://bootify.io/).

---

## Features

| Feature | Details |
|---|---|
| 🦀 **Pure Rust / WASM** | All template parsing and rendering runs in-browser — zero server roundtrips |
| 📋 **Dynamic forms** | `baker.yaml` drives the form UI automatically (text, bool toggle, select, multi-select chips, JSON/YAML textarea) |
| 🔁 **`ask_if` support** | Fields show/hide based on Jinja2 conditions evaluated live against current form values |
| 🌐 **GitHub templates** | Fetch any public GitHub repo (root or sub-path) by URL |
| 👀 **File preview** | Browse generated files with a syntax-highlighted viewer before downloading |
| 📦 **ZIP download** | Download the complete project as a `.zip` — no uploads, no cookies |

---

## Prerequisites

```bash
# Rust + wasm target
rustup target add wasm32-unknown-unknown

# Dioxus CLI (already installed if you have dx 0.7+)
cargo install dioxus-cli
```

---

## Development server

```bash
cd crates/baker-web
dx serve
```

Open <http://localhost:8080> in your browser.  
Hot-reload is enabled — editing any `.rs` or `.css` file rebuilds automatically.

---

## Production build

```bash
cd crates/baker-web
dx build --release
```

The output lands in `crates/baker-web/dist/`.  
It's a fully static site — drop it on any CDN (GitHub Pages, Cloudflare Pages, Netlify, etc.).

---

## Project structure

```
crates/baker-web/
├── Cargo.toml          # dependencies (dioxus, minijinja, serde_yaml, gloo-net, zip …)
├── Dioxus.toml         # build config, asset paths
├── index.html          # HTML shell that Dioxus mounts into
├── assets/
│   ├── style.css       # full dark-panel UI stylesheet
│   └── favicon.svg
└── src/
    ├── main.rs         # entry point
    ├── lib.rs          # module declarations
    ├── app.rs          # root component + global state (Signals)
    ├── config.rs       # baker.yaml parser (WASM-safe, no native deps)
    ├── renderer.rs     # Jinja2 rendering via minijinja
    ├── github.rs       # GitHub API + raw content fetcher (gloo-net)
    ├── templates.rs    # community template registry + URL parser
    ├── models.rs       # shared types (TemplateEntry, Loadable<T>, …)
    ├── zip.rs          # in-memory ZIP + browser download trigger
    └── components/
        ├── topbar.rs
        ├── template_panel.rs   # left panel: search, custom URL, list
        ├── form_panel.rs       # centre panel: dynamic form
        └── preview_panel.rs    # right panel: file tree + viewer + download
```

---

## Adding community templates

Edit [`src/templates.rs`](src/templates.rs) and add a `TemplateEntry` to the
`community_templates()` function:

```rust
TemplateEntry {
    name: "My Template".into(),
    description: "A great starting point.".into(),
    tags: vec!["rust".into(), "cli".into()],
    owner: "github-org".into(),
    repo: "my-baker-template".into(),
    branch: "main".into(),
    path: "".into(),  // or "examples/demo" for a sub-directory
},
```

The template just needs a `baker.yaml` at its root (or the specified `path`).
