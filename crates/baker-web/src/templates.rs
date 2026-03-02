//! Hardcoded community-template registry.
//!
//! Each entry points at a GitHub repo (+ optional sub-path) that contains a
//! valid `baker.yaml`.

use crate::models::TemplateEntry;

pub fn community_templates() -> Vec<TemplateEntry> {
    vec![
        // ── Baker built-in examples ─────────────────────────────────────────
        TemplateEntry {
            name: "Demo (Python)".into(),
            description:
                "A minimal Python project with optional tests, author, and slug.".into(),
            tags: vec!["python".into(), "starter".into()],
            owner: "aliev".into(),
            repo: "baker".into(),
            branch: "main".into(),
            path: "examples/demo".into(),
        },
        TemplateEntry {
            name: "Hooks Demo".into(),
            description:
                "Template demonstrating pre/post hook execution with licence selection."
                    .into(),
            tags: vec!["hooks".into(), "licence".into()],
            owner: "aliev".into(),
            repo: "baker".into(),
            branch: "main".into(),
            path: "examples/hooks".into(),
        },
        TemplateEntry {
            name: "Loop Templates".into(),
            description:
                "Showcases baker loop expressions in filenames and nested conditionals."
                    .into(),
            tags: vec!["advanced".into(), "loop".into()],
            owner: "aliev".into(),
            repo: "baker".into(),
            branch: "main".into(),
            path: "examples/loop".into(),
        },
        TemplateEntry {
            name: "Custom Filters".into(),
            description: "Demonstrates all baker built-in Jinja2 filters.".into(),
            tags: vec!["filters".into(), "jinja2".into()],
            owner: "aliev".into(),
            repo: "baker".into(),
            branch: "main".into(),
            path: "examples/filters".into(),
        },
        TemplateEntry {
            name: "Macro Imports".into(),
            description:
                "Shows how to split templates across multiple files using imports.".into(),
            tags: vec!["macros".into(), "advanced".into()],
            owner: "aliev".into(),
            repo: "baker".into(),
            branch: "main".into(),
            path: "examples/import".into(),
        },
        // ── Community templates ─────────────────────────────────────────────
        TemplateEntry {
            name: "Python Package".into(),
            description:
                "Modern Python package with pyproject.toml, ruff, pytest and CI.".into(),
            tags: vec!["python".into(), "package".into(), "ci".into()],
            owner: "aliev".into(),
            repo: "baker-template-python-package".into(),
            branch: "main".into(),
            path: "".into(),
        },
        TemplateEntry {
            name: "Rust CLI".into(),
            description:
                "Rust command-line tool scaffolded with clap and GitHub Actions CI."
                    .into(),
            tags: vec!["rust".into(), "cli".into()],
            owner: "aliev".into(),
            repo: "baker-template-rust-cli".into(),
            branch: "main".into(),
            path: "".into(),
        },
        TemplateEntry {
            name: "FastAPI Service".into(),
            description:
                "Production-ready FastAPI micro-service with Docker, tests, and OpenAPI."
                    .into(),
            tags: vec!["python".into(), "fastapi".into(), "docker".into()],
            owner: "aliev".into(),
            repo: "baker-template-fastapi".into(),
            branch: "main".into(),
            path: "".into(),
        },
        TemplateEntry {
            name: "TypeScript Library".into(),
            description: "TypeScript library with Vite, Vitest, and semantic-release."
                .into(),
            tags: vec!["typescript".into(), "library".into()],
            owner: "aliev".into(),
            repo: "baker-template-ts-lib".into(),
            branch: "main".into(),
            path: "".into(),
        },
    ]
}

/// Parse a GitHub URL into a `TemplateEntry`.
///
/// Accepts formats:
///  - `https://github.com/owner/repo`
///  - `https://github.com/owner/repo/tree/branch`
///  - `https://github.com/owner/repo/tree/branch/sub/path`
pub fn parse_github_url(url: &str) -> Result<TemplateEntry, String> {
    let url = url.trim().trim_end_matches('/');

    // Strip protocol + host
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("github.com/"))
        .ok_or_else(|| {
            "Only GitHub URLs are supported (https://github.com/owner/repo)".to_string()
        })?;

    let parts: Vec<&str> = path.splitn(5, '/').collect();

    match parts.as_slice() {
        [owner, repo] => Ok(TemplateEntry {
            name: repo.to_string(),
            description: format!("{}/{}", owner, repo),
            tags: vec!["custom".into()],
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: "main".into(),
            path: "".into(),
        }),
        [owner, repo, "tree", branch] => Ok(TemplateEntry {
            name: repo.to_string(),
            description: format!("{}/{} @ {}", owner, repo, branch),
            tags: vec!["custom".into()],
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
            path: "".into(),
        }),
        [owner, repo, "tree", branch, sub_path] => Ok(TemplateEntry {
            name: format!("{}/{}", repo, sub_path),
            description: format!("{}/{}/{} @ {}", owner, repo, sub_path, branch),
            tags: vec!["custom".into()],
            owner: owner.to_string(),
            repo: repo.to_string(),
            branch: branch.to_string(),
            path: sub_path.to_string(),
        }),
        _ => Err(format!("Cannot parse GitHub URL: {url}")),
    }
}
