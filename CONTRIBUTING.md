# Contributing to Baker

Thank you for your interest in the Baker project! This document contains guidelines for contributing to the project.

## 📋 Table of Contents

- [Conventional Commits](#conventional-commits)
- [Commit Examples](#commit-examples)
- [Release Process](#release-process)
- [Automatic Changelog Generation](#automatic-changelog-generation)
- [Development Environment Setup](#development-environment-setup)

## 🔧 Conventional Commits

The project uses [Conventional Commits](https://www.conventionalcommits.org/) for automatic changelog generation and version determination.

### Commit Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Supported Types

| Type       | Description                                 | Version Impact |
| ---------- | ------------------------------------------- | -------------- |
| `feat`     | New feature                                 | Minor bump     |
| `fix`      | Bug fix                                     | Patch bump     |
| `docs`     | Documentation changes                       | Patch bump     |
| `style`    | Formatting, missing semicolons, etc.        | Patch bump     |
| `refactor` | Code refactoring                            | Patch bump     |
| `perf`     | Performance improvements                    | Patch bump     |
| `test`     | Adding tests                                | Patch bump     |
| `chore`    | Changes to build process or auxiliary tools | Patch bump     |
| `ci`       | CI/CD changes                               | Patch bump     |
| `revert`   | Reverting changes                           | Patch bump     |

### Breaking Changes

For breaking changes, add `!` after the type or add `BREAKING CHANGE:` in the footer:

```
feat!: remove support for Node 14
```

or

```
feat: add new API endpoint

BREAKING CHANGE: old API endpoint removed
```

Breaking changes always trigger a **Major** version bump.

## 📝 Commit Examples

### ✅ Good Examples

```bash
# New features
feat: add support for YAML templates
feat(cli): add --dry-run flag for testing templates
feat(hooks): add pre-commit hook support

# Bug fixes
fix: handle empty template files correctly
fix(config): resolve path resolution on Windows
fix(validation): prevent infinite loop in conditional questions

# Documentation
docs: update README with new installation methods
docs(api): add examples for template functions

# Refactoring
refactor: extract template rendering logic
refactor(dialoguer): simplify prompt interface

# Tests
test: add integration tests for hook execution
test(cli): add tests for argument parsing

# Maintenance
chore: bump dependencies to latest versions
chore(release): prepare for v0.10.0
ci: add coverage reporting to GitHub Actions
```

### ❌ Bad Examples

```bash
# Too generic
git commit -m "fix stuff"
git commit -m "update"
git commit -m "changes"

# Don't follow format
git commit -m "Fixed the bug with templates"
git commit -m "Added new feature for hooks"
git commit -m "Updated documentation"
```

## 🚀 Release Process

### Prerequisites

1. Install git-cliff:

   ```bash
   cargo install git-cliff
   ```

2. Ensure you're on the main branch and it's up-to-date:

   ```bash
   git checkout main
   git pull origin main
   ```

3. Ensure the working directory is clean:
   ```bash
   git status
   ```

### Release Steps

#### 1. Determine Version Type

Based on commits since the last release:

- **Patch** (0.9.0 → 0.9.1): bug fixes and minor changes only
- **Minor** (0.9.0 → 0.10.0): new features, backward compatible
- **Major** (0.9.0 → 1.0.0): breaking changes

#### 2. Check Unreleased Changes

```bash
git-cliff --unreleased
```

#### 3. Update Changelog

```bash
# For patch version
git-cliff --output CHANGELOG.md

# Or for specific version
git-cliff --tag v0.10.0 --output CHANGELOG.md
```

#### 4. Update Version in Cargo.toml

```bash
# For example, for version 0.10.0
sed -i 's/version = ".*"/version = "0.10.0"/' Cargo.toml
```

#### 5. Commit Changes

```bash
git add CHANGELOG.md Cargo.toml
git commit -m "chore(release): prepare for v0.10.0"
```

#### 6. Create and Push Tag

```bash
git tag v0.10.0
git push origin main
git push origin v0.10.0
```

## 📚 Automatic Changelog Generation

The project uses [git-cliff](https://git-cliff.org/) for automatic changelog generation based on conventional commits.

### Configuration

Configuration is located in the `cliff.toml` file. Main settings:

- **Conventional commits**: enabled
- **Breaking changes**: always trigger major bump
- **Features**: always trigger minor bump
- **Grouping**: by commit type with emojis

### GitHub Actions

When creating a tag, the following happens automatically:

1. **changelog.yaml**: generates changelog for GitHub release description
2. **release.yaml**: creates release with built artifacts

## 🛠 Development Environment Setup

### Requirements

- Rust 1.70+
- Git

### Clone and Build

```bash
git clone https://github.com/aliev/baker.git
cd baker
cargo build
cargo test
```

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# With output
cargo test -- --nocapture
```

### Linting and Formatting

```bash
# Formatting
cargo fmt

# Linting
cargo clippy

# Check formatting
cargo fmt -- --check
```

## 🎯 Useful git-cliff Commands

```bash
# Show unreleased changes
git-cliff --unreleased

# Show latest release only
git-cliff --latest

# Show specific version
git-cliff --tag v0.9.0

# Show bump version for unreleased changes
git-cliff --bumped-version

# Update changelog
git-cliff --output CHANGELOG.md

# Generate for specific range
git-cliff v0.8.0..v0.9.0
```

## 📊 Commit Usage Examples

### Pull Request

Ensure the PR title follows conventional commits:

```
feat(templates): add support for nested includes
fix(cli): resolve argument parsing for Windows paths
docs: add contribution guidelines
```

### Merge Commits

When merging, use "Squash and merge" with proper title:

```
feat: add YAML template validation (#45)
```

## ❓ Questions

If you have questions about the release process or conventional commits, create an issue in the repository or contact the maintainers.

---

**Thank you for contributing to Baker! 🚀**
