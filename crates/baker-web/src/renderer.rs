//! Jinja2-compatible template rendering via minijinja (WASM-safe)

use indexmap::IndexMap;
use minijinja::Environment;

/// Render a single template string with the provided context JSON.
pub fn render(template: &str, context: &serde_json::Value) -> Result<String, String> {
    let mut env = Environment::new();
    register_filters(&mut env);
    env.add_template("__t__", template).map_err(|e| e.to_string())?;
    let tmpl = env.get_template("__t__").map_err(|e| e.to_string())?;
    tmpl.render(context).map_err(|e| e.to_string())
}

/// Render a path expression.
///
/// Handles `{% for item in list %}{{ item }}.txt{% endfor %}` by treating
/// each rendered iteration as a separate path — returns a `Vec<String>` of
/// non-empty, trimmed results.
pub fn render_path(
    path: &str,
    context: &serde_json::Value,
) -> Result<Vec<String>, String> {
    // Wrap a `for` loop body so we can capture multi-path outputs.
    // Simpler approach: just render, then split on a sentinel.
    // Introduce newline between loop iterations using a modified template.
    let augmented = inject_path_newlines(path);
    let rendered = render(&augmented, context)?;
    let paths: Vec<String> = rendered
        .split('\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Ok(if paths.is_empty() { vec![] } else { paths })
}

/// Apply `render_project`: given a map of `raw_path -> raw_content` and a JSON
/// context, return the rendered output files.
pub fn render_project(
    template_suffix: &str,
    files: &IndexMap<String, String>,
    context: &serde_json::Value,
) -> Result<IndexMap<String, String>, String> {
    let mut output: IndexMap<String, String> = IndexMap::new();

    for (raw_path, raw_content) in files {
        // Skip config files
        if matches!(raw_path.as_str(), "baker.yaml" | "baker.yml" | "baker.json") {
            continue;
        }

        // Render the path (may produce 0 or multiple expanded paths)
        let expanded_paths = render_path(raw_path, context).unwrap_or_default();
        if expanded_paths.is_empty() {
            continue;
        }

        let is_template = raw_path.ends_with(template_suffix);

        for ep in expanded_paths {
            // Strip template suffix from the output path
            let out_path = if is_template {
                ep.trim_end_matches(template_suffix).to_string()
            } else {
                ep.clone()
            };

            if out_path.is_empty() {
                continue;
            }

            let out_content = if is_template {
                render(raw_content, context).unwrap_or_else(|_| raw_content.clone())
            } else {
                raw_content.clone()
            };

            output.insert(out_path, out_content);
        }
    }

    Ok(output)
}

// ─── Path newline injection ────────────────────────────────────────────────────
//
// Baker paths of the form:
//   `{%for item in items%}{{ item.name }}.md.baker.j2{%endfor%}`
//
// We need each iteration to be on its own line so `render_path` can split on
// `\n`.  We insert a `\n` at the end of the loop body by rewriting the closing
// tag to `{{ '\n' }}{% endfor %}`.
//
fn inject_path_newlines(path: &str) -> String {
    // Simple heuristic: if the path contains a `{%for` block, inject a
    // newline before `{%endfor%}` / `{% endfor %}`.
    if !path.contains("{%") && !path.contains("{% ") {
        return path.to_string();
    }
    // Replace `{%endfor%}` variants with a newline sentinel before the tag.
    let replaced = path
        .replace("{%- endfor -%}", "\n{%- endfor -%}")
        .replace("{%- endfor%}", "\n{%- endfor%}")
        .replace("{%endfor-%}", "\n{%endfor-%}")
        .replace("{%endfor%}", "\n{%endfor%}")
        .replace("{%- endfor -%}", "\n{%- endfor -%}")
        .replace("{% endfor %}", "\n{% endfor %}")
        .replace("{% endfor-%}", "\n{% endfor-%}")
        .replace("{%- endfor %}", "\n{%- endfor %}");
    replaced
}

// ─── Custom Jinja2 filters ─────────────────────────────────────────────────────

fn register_filters(env: &mut Environment) {
    env.add_filter("slugify", |v: String| {
        cruet::to_snake_case(&v).replace('_', "-").to_lowercase()
    });
    env.add_filter("snake_case", |v: String| cruet::to_snake_case(&v));
    env.add_filter("camel_case", |v: String| cruet::to_camel_case(&v));
    env.add_filter("pascal_case", |v: String| cruet::to_class_case(&v));
    env.add_filter("kebab_case", |v: String| cruet::to_kebab_case(&v));
    env.add_filter("title_case", |v: String| cruet::to_title_case(&v));
    env.add_filter("lower", |v: String| v.to_lowercase());
    env.add_filter("upper", |v: String| v.to_uppercase());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_render() {
        let ctx = json!({"name": "MyProject"});
        assert_eq!(render("{{ name }}", &ctx).unwrap(), "MyProject");
    }

    #[test]
    fn test_conditional_path() {
        let ctx = json!({"use_tests": true});
        let paths = render_path("{% if use_tests %}tests{% endif %}", &ctx).unwrap();
        assert_eq!(paths, vec!["tests"]);

        let ctx2 = json!({"use_tests": false});
        let paths2 = render_path("{% if use_tests %}tests{% endif %}", &ctx2).unwrap();
        assert!(paths2.is_empty());
    }

    #[test]
    fn test_filter_snake_case() {
        let ctx = json!({"name": "My Cool Project"});
        assert_eq!(render("{{ name|snake_case }}", &ctx).unwrap(), "my_cool_project");
    }
}
