use baker::conflict::{apply_conflict_markers, ConflictStyle};
use baker::renderer::{MiniJinjaRenderer, TemplateRenderer};
use serde_json::json;
use std::path::Path;

fn main() {
    divan::main();
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

#[divan::bench]
fn render_simple_template() -> String {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({"name": "world"});
    renderer.render("Hello, {{ name }}!", &ctx, None).unwrap()
}

#[divan::bench]
fn render_template_with_filters() -> String {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({"project_name": "my awesome project"});
    let tpl = "\
        Name: {{ project_name }}\n\
        Camel: {{ project_name | camel_case }}\n\
        Kebab: {{ project_name | kebab_case }}\n\
        Pascal: {{ project_name | pascal_case }}\n\
        Snake: {{ project_name | snake_case }}\n\
        Train: {{ project_name | train_case }}";
    renderer.render(tpl, &ctx, None).unwrap()
}

#[divan::bench]
fn render_template_with_conditionals() -> String {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({
        "use_docker": true,
        "use_ci": true,
        "ci_provider": "github",
        "project_name": "demo",
        "features": ["auth", "logging", "metrics"]
    });
    let tpl = "\
        project: {{ project_name }}\n\
        {% if use_docker %}docker: enabled{% endif %}\n\
        {% if use_ci %}\n\
        ci: {{ ci_provider }}\n\
        {% endif %}\n\
        {% for feature in features %}\n\
        - {{ feature }}\n\
        {% endfor %}";
    renderer.render(tpl, &ctx, None).unwrap()
}

#[divan::bench]
fn render_large_context() -> String {
    let renderer = MiniJinjaRenderer::new();
    let mut obj = serde_json::Map::new();
    for i in 0..50 {
        obj.insert(format!("var_{i}"), serde_json::Value::String(format!("value_{i}")));
    }
    let ctx = serde_json::Value::Object(obj);
    let tpl =
        (0..50).map(|i| format!("{{{{ var_{i} }}}}")).collect::<Vec<_>>().join("\n");
    renderer.render(&tpl, &ctx, None).unwrap()
}

// ---------------------------------------------------------------------------
// Path rendering
// ---------------------------------------------------------------------------

#[divan::bench]
fn render_path_simple() -> String {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({"service": "api", "name": "config"});
    renderer.render_path(Path::new("src/{{ service }}/{{ name }}.rs"), &ctx).unwrap()
}

#[divan::bench]
fn render_path_nested() -> String {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({
        "org": "acme",
        "project": "widgets",
        "module": "auth"
    });
    renderer
        .render_path(Path::new("{{ org }}/{{ project }}/src/{{ module }}/mod.rs"), &ctx)
        .unwrap()
}

// ---------------------------------------------------------------------------
// Expression evaluation
// ---------------------------------------------------------------------------

#[divan::bench]
fn execute_expression_simple() -> bool {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({"use_docker": true});
    renderer.execute_expression("use_docker", &ctx).unwrap()
}

#[divan::bench]
fn execute_expression_complex() -> bool {
    let renderer = MiniJinjaRenderer::new();
    let ctx = json!({
        "language": "rust",
        "use_ci": true,
        "version": 3
    });
    renderer
        .execute_expression("language == 'rust' and use_ci and version >= 2", &ctx)
        .unwrap()
}

// ---------------------------------------------------------------------------
// Conflict markers
// ---------------------------------------------------------------------------

#[divan::bench]
fn conflict_markers_identical() -> String {
    let content = "line1\nline2\nline3\nline4\nline5\n";
    apply_conflict_markers(content, content, ConflictStyle::Git)
}

#[divan::bench]
fn conflict_markers_partial_diff() -> String {
    let existing = "header\nline2\noriginal middle\nline4\nfooter\n";
    let updated = "header\nline2\nupdated middle\nline4\nfooter\n";
    apply_conflict_markers(existing, updated, ConflictStyle::Git)
}

#[divan::bench]
fn conflict_markers_large_files() -> String {
    let existing: String =
        (0..500).map(|i| format!("line {i}: original content here\n")).collect();
    let mut updated = existing.clone();
    // Change lines in the middle
    updated = updated
        .replace("line 250: original content here", "line 250: UPDATED content here");
    apply_conflict_markers(&existing, &updated, ConflictStyle::Git)
}

// ---------------------------------------------------------------------------
// Renderer construction
// ---------------------------------------------------------------------------

#[divan::bench]
fn renderer_construction() -> MiniJinjaRenderer {
    MiniJinjaRenderer::new()
}
