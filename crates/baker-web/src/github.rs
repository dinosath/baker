//! GitHub API client — fetches baker.yaml and template files using the
//! browser's native `fetch` via `gloo-net`.

use crate::models::TemplateEntry;
use gloo_net::http::Request;
use indexmap::IndexMap;
use serde::Deserialize;

// ─── GitHub tree API types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GitTree {
    tree: Vec<TreeEntry>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Debug, Deserialize)]
struct TreeEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Fetch the `baker.yaml` (or `.yml` / `.json`) from a template entry.
/// Tries `baker.yaml` then `baker.yml` then `baker.json`.
pub async fn fetch_baker_yaml(entry: &TemplateEntry) -> Result<String, String> {
    let base = entry.raw_base_url();
    for name in &["baker.yaml", "baker.yml", "baker.json"] {
        let url = format!("{base}{name}");
        match fetch_text(&url).await {
            Ok(text) => return Ok(text),
            Err(_) => continue,
        }
    }
    Err(format!(
        "No baker.yaml found in {}/{}:{}",
        entry.owner,
        entry.repo,
        if entry.path.is_empty() {
            "(root)".into()
        } else {
            entry.path.clone()
        }
    ))
}

/// Fetch all template files for the entry, returning `path -> content`.
///
/// Uses the GitHub tree API to list files, then fetches each one in sequence.
/// Files inside `.bakerignore` lines are not filtered here (that's renderer logic).
pub async fn fetch_template_files(
    entry: &TemplateEntry,
) -> Result<IndexMap<String, String>, String> {
    let tree_url = entry.tree_api_url();
    let tree: GitTree = fetch_json(&tree_url)
        .await
        .map_err(|e| format!("GitHub tree API error: {e}"))?;

    if tree.truncated {
        log::warn!(
            "GitHub tree response is truncated for {}/{}",
            entry.owner,
            entry.repo
        );
    }

    let prefix = if entry.path.is_empty() {
        String::new()
    } else {
        format!("{}/", entry.path.trim_matches('/'))
    };

    let blob_paths: Vec<String> = tree
        .tree
        .into_iter()
        .filter(|e| e.entry_type == "blob")
        .filter(|e| e.path.starts_with(&prefix))
        .map(|e| e.path)
        .collect();

    let base = entry.raw_base_url();
    let mut files: IndexMap<String, String> = IndexMap::new();

    for full_path in blob_paths {
        // Strip the template sub-path prefix so paths are relative to
        // the template root.
        let rel_path = if prefix.is_empty() {
            full_path.clone()
        } else {
            full_path
                .strip_prefix(&prefix)
                .unwrap_or(&full_path)
                .to_string()
        };

        // Fetch raw content — skip binary files silently (non-UTF8)
        let raw_url = format!("{base}{rel_path}");
        match fetch_text(&raw_url).await {
            Ok(content) => {
                files.insert(rel_path, content);
            }
            Err(e) => {
                log::warn!("Skipping {rel_path}: {e}");
            }
        }
    }

    Ok(files)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

async fn fetch_text(url: &str) -> Result<String, String> {
    let resp = Request::get(url)
        .header("Accept", "text/plain, */*")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {url}", resp.status()));
    }

    resp.text().await.map_err(|e| e.to_string())
}

async fn fetch_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    let resp = Request::get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.ok() {
        return Err(format!("HTTP {}: {url}", resp.status()));
    }

    resp.json::<T>().await.map_err(|e| e.to_string())
}
