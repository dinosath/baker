# Architecture Overview

Baker follows a layered flow that starts with command parsing and ends with generating files on disk. Each layer owns a focused responsibility so contributors can reason about changes without touching unrelated subsystems.

## Execution Pipeline
1. **CLI (`src/cli`)** – `main.rs` delegates to `cli::runner::run`. The runner validates command-line arguments, prepares output directories, and orchestrates the remaining steps.
2. **Template Acquisition (`src/loader`)** – `get_template` resolves a local path or Git repository into a working template directory.
3. **Configuration (`src/config`)** – `Config::load_config` parses `baker.yaml`/`baker.json`, returning a validated `ConfigV1`. Configuration controls template suffixes, loop separators, hooks, and user questions.
4. **Q&A (`src/prompt`)** – `prompt::handler::PromptHandler` drives interactive collection of answers using `dialoguer`, honoring defaults, validation rules, and `--non-interactive` mode.
5. **Pre-render Hook (`src/cli`)** – When present, `hooks/pre_render` receives the collected answers and can return additional JSON answers before rendering begins.
6. **Template Engine (`src/renderer`)** – `MiniJinjaRenderer` renders file content, filenames, and hook names using the collected answers.
7. **Processing (`src/template`)** – `TemplateProcessor` evaluates each template entry, decides whether to write, copy, create directories, or ignore paths (respecting `.bakerignore`), and expands loop-driven templates into multiple outputs.
8. **Filesystem Effects (`src/cli/processor.rs`)** – `FileProcessor` applies `TemplateOperation`s, prompting for overwrites unless suppressed by `--skip-confirms`. Hooks run before answer collection, before rendering, and after processing when provided.

## Supporting Modules
- **`src/constants.rs`** centralises exit codes, verbosity thresholds, and validation defaults.
- **`src/error.rs`** defines recoverable and fatal error types with contextual messages.
- **`src/ext`** hosts helper traits (for example, `PathExt`) to keep core modules uncluttered.

## Data Flow Highlights
- Answers collected from prompts can be enriched by the pre-render hook before they feed into renderers, enabling templated defaults, conditional paths, and generated structured data.
- Dry-run mode short-circuits filesystem writes while still exercising rendering and logging for safe previews.
- Skip-confirm flags toggle confirmation prompts independently for overwriting files and executing hooks.

Understanding these boundaries helps isolate changes: adjust prompting logic inside `prompt`, extend template behaviour in `template`, and keep the runner focused on sequencing the pipeline.
