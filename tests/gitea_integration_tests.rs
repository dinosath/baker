//! Integration tests for Git repository cloning using Gitea testcontainer.
//!
//! These tests verify that the GitLoader can successfully clone repositories
//! from a real Git server (Gitea running in a container).
//!
//! All tests share a single Gitea instance for efficiency.
//!
//! Uses pure gix for all local git operations (clone, checkout).
//! Uses Gitea REST API for creating content on the server (since gix lacks push support).
//!
//! Run these tests with: `cargo test --test gitea_integration_tests -- --ignored`

use baker::cli::SkipConfirm::All;
use baker::cli::{run, Args};
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tempfile::TempDir;
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::SyncRunner,
    GenericImage, ImageExt,
};

// ============================================================================
// Git operations module - pure gix implementation for clone and checkout
// ============================================================================
mod git_ops {
    use gix::progress::Discard;
    use std::fs;
    use std::path::Path;

    /// Clone a repository using gix
    pub fn clone_repo(
        url: &str,
        target_path: &Path,
    ) -> Result<gix::Repository, Box<dyn std::error::Error>> {
        let url = gix::url::parse(url.into())?;
        let mut clone = gix::clone::PrepareFetch::new(
            url,
            target_path,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            gix::open::Options::isolated(),
        )?;

        let (mut checkout, _) =
            clone.fetch_then_checkout(Discard, &gix::interrupt::IS_INTERRUPTED)?;
        let (repo, _) = checkout.main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED)?;

        Ok(repo.into_sync().to_thread_local())
    }

    /// Clone a repository at a specific branch using gix
    pub fn clone_repo_branch(
        url: &str,
        branch: &str,
        target_path: &Path,
    ) -> Result<gix::Repository, Box<dyn std::error::Error>> {
        let url = gix::url::parse(url.into())?;
        let mut clone = gix::clone::PrepareFetch::new(
            url,
            target_path,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            gix::open::Options::isolated(),
        )?
        .with_ref_name(Some(branch))?;

        let (mut checkout, _) =
            clone.fetch_then_checkout(Discard, &gix::interrupt::IS_INTERRUPTED)?;
        let (repo, _) = checkout.main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED)?;

        Ok(repo.into_sync().to_thread_local())
    }

    /// Get current branch name from a gix repository
    pub fn get_branch(repo: &gix::Repository) -> Result<String, Box<dyn std::error::Error>> {
        let head = repo.head()?;
        let name = head
            .referent_name()
            .map(|n| n.shorten().to_string())
            .unwrap_or_else(|| "HEAD".to_string());
        Ok(name)
    }

    /// Checkout a specific ref and update working directory using gix
    pub fn checkout(
        repo: &gix::Repository,
        ref_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Find the reference (try refs/tags/ first, then refs/heads/, then direct)
        let mut reference = repo
            .find_reference(&format!("refs/tags/{}", ref_name))
            .or_else(|_| repo.find_reference(&format!("refs/heads/{}", ref_name)))
            .or_else(|_| repo.find_reference(ref_name))?;

        let commit = reference.peel_to_commit()?;
        let tree = commit.tree()?;

        let workdir = repo.workdir().ok_or("No working directory")?;

        // Checkout tree to workdir
        checkout_tree_to_workdir(repo, &tree, workdir)?;

        Ok(())
    }

    /// Checkout tree contents to working directory
    fn checkout_tree_to_workdir(
        repo: &gix::Repository,
        tree: &gix::Tree<'_>,
        workdir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in tree.iter() {
            let entry = entry?;
            let path = workdir.join(entry.filename().to_string());

            match entry.mode().kind() {
                gix::objs::tree::EntryKind::Blob | gix::objs::tree::EntryKind::BlobExecutable => {
                    let blob = repo.find_blob(entry.oid())?;
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&path, blob.data.as_slice())?;
                }
                gix::objs::tree::EntryKind::Tree => {
                    fs::create_dir_all(&path)?;
                    let subtree = repo.find_tree(entry.oid())?;
                    checkout_tree_to_workdir(repo, &subtree, &path)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// ============================================================================
// Gitea API module - for creating content on the server
// ============================================================================
mod gitea_api {
    use base64::Engine;
    use reqwest::blocking::Client;
    use serde::Deserialize;
    use serde_json::json;
    use std::path::Path;

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct FileResponse {
        pub commit: CommitInfo,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    pub struct CommitInfo {
        pub sha: String,
    }

    #[derive(Deserialize, Debug)]
    pub struct BranchResponse {
        pub commit: BranchCommit,
    }

    #[derive(Deserialize, Debug)]
    pub struct BranchCommit {
        pub id: String,
    }

    /// Create or update a file in the repository via Gitea API
    pub fn create_or_update_file(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        file_path: &str,
        content: &str,
        message: &str,
        branch: &str,
        sha: Option<&str>, // For updates, provide existing file SHA
    ) -> Result<FileResponse, Box<dyn std::error::Error>> {
        let encoded_content = base64::engine::general_purpose::STANDARD.encode(content);

        let mut payload = json!({
            "content": encoded_content,
            "message": message,
            "branch": branch
        });

        if let Some(existing_sha) = sha {
            payload["sha"] = json!(existing_sha);
        }

        let response = client
            .post(&format!(
                "{}/api/v1/repos/{}/{}/contents/{}",
                base_url, owner, repo, file_path
            ))
            .basic_auth(username, Some(password))
            .json(&payload)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "Failed to create/update file {}: {} - {}",
                file_path, status, body
            )
            .into());
        }

        let file_response: FileResponse = response.json()?;
        Ok(file_response)
    }

    /// Get the SHA of a file (needed for updates)
    pub fn get_file_sha(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        file_path: &str,
        branch: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let response = client
            .get(&format!(
                "{}/api/v1/repos/{}/{}/contents/{}?ref={}",
                base_url, owner, repo, file_path, branch
            ))
            .basic_auth(username, Some(password))
            .send()?;

        if !response.status().is_success() {
            return Err(format!("File {} not found", file_path).into());
        }

        #[derive(Deserialize)]
        struct FileInfo {
            sha: String,
        }

        let file_info: FileInfo = response.json()?;
        Ok(file_info.sha)
    }

    /// Create a branch from a specific commit
    pub fn create_branch(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        branch_name: &str,
        source_branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "new_branch_name": branch_name,
            "old_ref_name": source_branch
        });

        let response = client
            .post(&format!(
                "{}/api/v1/repos/{}/{}/branches",
                base_url, owner, repo
            ))
            .basic_auth(username, Some(password))
            .json(&payload)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "Failed to create branch {}: {} - {}",
                branch_name, status, body
            )
            .into());
        }

        Ok(())
    }

    /// Get the commit SHA for a branch
    pub fn get_branch_sha(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        branch: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let response = client
            .get(&format!(
                "{}/api/v1/repos/{}/{}/branches/{}",
                base_url, owner, repo, branch
            ))
            .basic_auth(username, Some(password))
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "Failed to get branch {}: {} - {}",
                branch, status, body
            )
            .into());
        }

        let branch_info: BranchResponse = response.json()?;
        Ok(branch_info.commit.id)
    }

    /// Create a tag at a specific commit
    pub fn create_tag(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        tag_name: &str,
        target_sha: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "tag_name": tag_name,
            "target": target_sha,
            "message": message
        });

        let response = client
            .post(&format!(
                "{}/api/v1/repos/{}/{}/tags",
                base_url, owner, repo
            ))
            .basic_auth(username, Some(password))
            .json(&payload)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "Failed to create tag {}: {} - {}",
                tag_name, status, body
            )
            .into());
        }

        Ok(())
    }

    /// Upload multiple files to create a repository's initial content
    pub fn upload_directory(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        local_dir: &Path,
        branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        upload_directory_recursive(
            client, base_url, owner, repo, username, password, local_dir, local_dir, branch,
        )
    }

    fn upload_directory_recursive(
        client: &Client,
        base_url: &str,
        owner: &str,
        repo: &str,
        username: &str,
        password: &str,
        base_dir: &Path,
        current_dir: &Path,
        branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in std::fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip .git directory and any hidden files
            if file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                upload_directory_recursive(
                    client, base_url, owner, repo, username, password, base_dir, &path, branch,
                )?;
            } else {
                let relative_path = path
                    .strip_prefix(base_dir)?
                    .to_string_lossy()
                    .replace('\\', "/");
                let content = std::fs::read_to_string(&path)?;

                create_or_update_file(
                    client,
                    base_url,
                    owner,
                    repo,
                    username,
                    password,
                    &relative_path,
                    &content,
                    &format!("Add {}", relative_path),
                    branch,
                    None,
                )?;
            }
        }
        Ok(())
    }
}

// ============================================================================
// Test configuration
// ============================================================================

/// Gitea container image configuration
const GITEA_IMAGE: &str = "gitea/gitea";
const GITEA_TAG: &str = "1.25-rootless";
const GITEA_HTTP_PORT: u16 = 3000;

/// Test user credentials for Gitea
const TEST_USER: &str = "testuser";
const TEST_PASSWORD: &str = "Password123!";
const TEST_EMAIL: &str = "test@example.com";

/// Shared Gitea instance for all tests
static GITEA_INSTANCE: OnceLock<SharedGiteaEnv> = OnceLock::new();

/// Shared Gitea environment that persists across all tests
struct SharedGiteaEnv {
    #[allow(dead_code)]
    container: testcontainers::core::Container<GenericImage>,
    base_url: String,
    client: Client,
}

// Safety: The container and client are thread-safe for our read-only access patterns
unsafe impl Sync for SharedGiteaEnv {}
unsafe impl Send for SharedGiteaEnv {}

impl SharedGiteaEnv {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        eprintln!("Starting shared Gitea container...");

        let container = GenericImage::new(GITEA_IMAGE, GITEA_TAG)
            .with_exposed_port(GITEA_HTTP_PORT.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Starting new Web server"))
            .with_env_var("GITEA__security__INSTALL_LOCK", "true")
            .with_env_var("GITEA__database__DB_TYPE", "sqlite3")
            .with_env_var("GITEA__database__PATH", "/tmp/data/gitea.db")
            .with_env_var("GITEA__server__HTTP_PORT", GITEA_HTTP_PORT.to_string())
            .with_env_var("GITEA__server__DOMAIN", "localhost")
            .with_env_var("GITEA__server__ROOT_URL", "http://localhost:3000/")
            .with_env_var("GITEA__server__OFFLINE_MODE", "true")
            .with_env_var("GITEA__repository__DEFAULT_BRANCH", "main")
            .with_env_var("GITEA__service__DISABLE_REGISTRATION", "true")
            .with_env_var("GITEA__service__REQUIRE_SIGNIN_VIEW", "false")
            .with_env_var("GITEA__log__LEVEL", "Info")
            .start()?;

        eprintln!("Gitea container started successfully");

        let host_port = container
            .get_host_port_ipv4(GITEA_HTTP_PORT.tcp())
            .expect("Failed to get Gitea port");
        let base_url = format!("http://127.0.0.1:{}", host_port);

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Wait for Gitea to be ready
        if !wait_for_gitea_ready(&base_url, &client, 60) {
            return Err("Gitea failed to become ready".into());
        }

        // Create admin user via container exec
        create_admin_user(&container)?;

        // Small delay to ensure user is fully persisted
        std::thread::sleep(std::time::Duration::from_secs(1));

        eprintln!("Shared Gitea environment ready at {}", base_url);

        Ok(Self { container, base_url, client })
    }

    fn create_repo(&self, repo_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        create_gitea_repo(&self.base_url, &self.client, repo_name)
    }

    fn clone_url_with_auth(&self, repo_name: &str) -> String {
        let host_port = self.base_url.strip_prefix("http://127.0.0.1:").unwrap_or("3000");
        format!(
            "http://{}:{}@127.0.0.1:{}/{}/{}.git",
            TEST_USER, TEST_PASSWORD, host_port, TEST_USER, repo_name
        )
    }

    /// Upload directory contents to a repository via Gitea API
    fn upload_template(&self, repo_name: &str, local_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        gitea_api::upload_directory(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            local_dir,
            "main",
        )
    }

    /// Update a file in a repository via Gitea API
    fn update_file(
        &self, 
        repo_name: &str, 
        file_path: &str, 
        content: &str, 
        message: &str,
        branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sha = gitea_api::get_file_sha(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            file_path,
            branch,
        )?;

        gitea_api::create_or_update_file(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            file_path,
            content,
            message,
            branch,
            Some(&sha),
        )?;
        Ok(())
    }

    /// Create a branch via Gitea API
    fn create_branch(&self, repo_name: &str, branch_name: &str, source_branch: &str) -> Result<(), Box<dyn std::error::Error>> {
        gitea_api::create_branch(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            branch_name,
            source_branch,
        )
    }

    /// Create a tag via Gitea API
    fn create_tag(&self, repo_name: &str, tag_name: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sha = gitea_api::get_branch_sha(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            "main",
        )?;

        gitea_api::create_tag(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            tag_name,
            &sha,
            message,
        )
    }

    /// Create a file via Gitea API
    fn create_file(
        &self,
        repo_name: &str,
        file_path: &str,
        content: &str,
        message: &str,
        branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        gitea_api::create_or_update_file(
            &self.client,
            &self.base_url,
            TEST_USER,
            repo_name,
            TEST_USER,
            TEST_PASSWORD,
            file_path,
            content,
            message,
            branch,
            None,
        )?;
        Ok(())
    }
}

/// Gets or initializes the shared Gitea instance
fn get_shared_gitea() -> &'static SharedGiteaEnv {
    GITEA_INSTANCE.get_or_init(|| {
        SharedGiteaEnv::new().expect("Failed to create shared Gitea environment")
    })
}

// ============================================================================
// Test helper functions
// ============================================================================

/// Creates a sample baker template in the given directory
fn create_sample_template(dir: &Path) {
    let baker_yaml = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name
    default: "my project"

  version:
    type: str
    help: Enter version
    default: "0.1.0"
"#;
    fs::write(dir.join("baker.yaml"), baker_yaml).expect("Failed to write baker.yaml");

    let template_content = r#"# {{ project_name }}

Version: {{ version }}

This is a sample project.
"#;
    fs::write(dir.join("README.md.baker.j2"), template_content)
        .expect("Failed to write template file");

    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");
    fs::write(src_dir.join("main.txt"), "Hello from {{ project_name }}!")
        .expect("Failed to write main.txt");
}

/// Wait for Gitea API to be ready
fn wait_for_gitea_ready(base_url: &str, client: &Client, timeout_secs: u64) -> bool {
    eprintln!("Waiting for Gitea API at {}", base_url);
    for i in 0..timeout_secs {
        match client.get(&format!("{}/api/v1/version", base_url)).send() {
            Ok(response) => {
                if response.status().is_success() {
                    eprintln!("Gitea API is ready after {} seconds", i);
                    return true;
                }
                eprintln!(
                    "Gitea API responded with status {} (attempt {})",
                    response.status(),
                    i + 1
                );
            }
            Err(e) => {
                eprintln!("Gitea API not ready yet (attempt {}): {}", i + 1, e);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    eprintln!("Gitea API failed to become ready after {} seconds", timeout_secs);
    false
}

/// Creates the admin user using testcontainers exec
fn create_admin_user(
    container: &testcontainers::core::Container<GenericImage>,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Creating admin user via container exec");

    let cmd = ExecCommand::new(vec![
        "gitea",
        "admin",
        "user",
        "create",
        "--username",
        TEST_USER,
        "--password",
        TEST_PASSWORD,
        "--email",
        TEST_EMAIL,
        "--admin",
        "--must-change-password=false",
    ]);

    let mut result = container.exec(cmd)?;

    let exit_code = result.exit_code()?;
    if let Some(code) = exit_code {
        if code != 0 {
            let stderr = result.stderr_to_vec().unwrap_or_default();
            let stderr_str = String::from_utf8_lossy(&stderr);

            if !stderr_str.contains("already exists") {
                return Err(format!(
                    "Failed to create admin user (exit code {}): {}",
                    code, stderr_str
                )
                .into());
            }
        }
    }

    eprintln!("Admin user created successfully");
    Ok(())
}

/// Creates a repository in Gitea via API (with auto_init to create main branch)
fn create_gitea_repo(
    base_url: &str,
    client: &Client,
    repo_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    eprintln!("Creating repository: {}", repo_name);

    let create_repo_payload = json!({
        "name": repo_name,
        "description": "Test repository for baker integration tests",
        "private": false,
        "auto_init": true,  // Initialize with README to create main branch
        "default_branch": "main"
    });

    let response = client
        .post(&format!("{}/api/v1/user/repos", base_url))
        .basic_auth(TEST_USER, Some(TEST_PASSWORD))
        .json(&create_repo_payload)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to create repo: {} - {}", status, body).into());
    }

    // Small delay to ensure repo is fully initialized
    std::thread::sleep(std::time::Duration::from_millis(500));

    let repo_url = format!("{}/{}/{}.git", base_url, TEST_USER, repo_name);
    eprintln!("Repository created: {}", repo_url);
    Ok(repo_url)
}

/// Copies a directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ============================================================================
// Integration tests
// ============================================================================

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_git_clone_from_gitea() {
    let env = get_shared_gitea();

    // Create repository with auto_init
    let repo_name = "test-template-clone";
    let _repo_url = env.create_repo(repo_name).expect("Failed to create repository");
    let clone_url = env.clone_url_with_auth(repo_name);

    // Create sample template locally
    let template_dir = TempDir::new().expect("Failed to create temp dir");
    create_sample_template(template_dir.path());

    // Upload template to Gitea via API
    env.upload_template(repo_name, template_dir.path())
        .expect("Failed to upload template to Gitea");

    // Clone the repository using gix
    let work_dir = TempDir::new().expect("Failed to create work dir");
    let clone_path = work_dir.path().join("cloned-repo");

    git_ops::clone_repo(&clone_url, &clone_path).expect("Failed to clone repository");

    assert!(clone_path.exists(), "Cloned path does not exist");
    assert!(
        clone_path.join("baker.yaml").exists(),
        "baker.yaml not found in cloned repo"
    );
    assert!(
        clone_path.join("README.md.baker.j2").exists(),
        "README.md.baker.j2 not found in cloned repo"
    );
    assert!(
        clone_path.join("src").join("main.txt").exists(),
        "src/main.txt not found in cloned repo"
    );

    let baker_yaml_content = fs::read_to_string(clone_path.join("baker.yaml"))
        .expect("Failed to read baker.yaml");
    assert!(
        baker_yaml_content.contains("schemaVersion: v1"),
        "baker.yaml content mismatch"
    );
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_git_checkout_specific_branch() {
    let env = get_shared_gitea();

    let repo_name = "test-template-branch";
    let _repo_url = env.create_repo(repo_name).expect("Failed to create repository");
    let clone_url = env.clone_url_with_auth(repo_name);

    // Upload initial template
    let template_dir = TempDir::new().expect("Failed to create temp dir");
    create_sample_template(template_dir.path());
    env.upload_template(repo_name, template_dir.path())
        .expect("Failed to upload template");

    // Create feature branch via Gitea API
    env.create_branch(repo_name, "feature", "main")
        .expect("Failed to create feature branch");

    // Update baker.yaml on the feature branch
    let feature_content = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name (feature branch version)
    default: "my feature project"

  version:
    type: str
    help: Enter version
    default: "2.0.0"
"#;
    env.update_file(repo_name, "baker.yaml", feature_content, "Feature branch changes", "feature")
        .expect("Failed to update baker.yaml on feature branch");

    // Clone the feature branch using gix
    let work_dir = TempDir::new().expect("Failed to create work dir");
    let clone_path = work_dir.path().join("cloned-feature");

    let cloned_repo = git_ops::clone_repo_branch(&clone_url, "feature", &clone_path)
        .expect("Failed to clone feature branch");

    // Check we're on the feature branch
    let branch_name = git_ops::get_branch(&cloned_repo).expect("Failed to get current branch");
    assert_eq!(branch_name, "feature", "Not on feature branch");

    let baker_yaml_content = fs::read_to_string(clone_path.join("baker.yaml"))
        .expect("Failed to read baker.yaml");
    assert!(
        baker_yaml_content.contains("feature branch version"),
        "Content is not from feature branch"
    );
    assert!(
        baker_yaml_content.contains("2.0.0"),
        "Version should be 2.0.0 on feature branch"
    );
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_git_checkout_specific_tag() {
    let env = get_shared_gitea();

    let repo_name = "test-template-tag";
    let _repo_url = env.create_repo(repo_name).expect("Failed to create repository");
    let clone_url = env.clone_url_with_auth(repo_name);

    // Upload initial template
    let template_dir = TempDir::new().expect("Failed to create temp dir");
    create_sample_template(template_dir.path());
    env.upload_template(repo_name, template_dir.path())
        .expect("Failed to upload template");

    // Create tag at current state
    env.create_tag(repo_name, "v1.0.0", "Version 1.0.0")
        .expect("Failed to create tag");

    // Update baker.yaml on main after the tag
    let new_content = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name (after v1.0.0)
    default: "my newer project"
"#;
    env.update_file(repo_name, "baker.yaml", new_content, "Post v1.0.0 changes", "main")
        .expect("Failed to update baker.yaml");

    // Clone and checkout the tag using gix
    let work_dir = TempDir::new().expect("Failed to create work dir");
    let clone_path = work_dir.path().join("cloned-tag");

    let cloned_repo = git_ops::clone_repo(&clone_url, &clone_path)
        .expect("Failed to clone repository");

    // Checkout the tag
    git_ops::checkout(&cloned_repo, "v1.0.0").expect("Failed to checkout tag");

    let baker_yaml_content = fs::read_to_string(clone_path.join("baker.yaml"))
        .expect("Failed to read baker.yaml");
    assert!(
        baker_yaml_content.contains("my project"),
        "Content should be from v1.0.0 tag"
    );
    assert!(
        !baker_yaml_content.contains("after v1.0.0"),
        "Content should not include post-tag changes"
    );
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_jsonschema_file_template_from_gitea() {
    let env = get_shared_gitea();

    let repo_name = "jsonschema-file-template";
    let _repo_url = env.create_repo(repo_name).expect("Failed to create repository");
    let clone_url = env.clone_url_with_auth(repo_name);

    // Copy jsonschema_file template to temp dir
    let template_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let source_template = Path::new(&manifest_dir).join("tests/templates/jsonschema_file");
    copy_dir_recursive(&source_template, template_dir.path())
        .expect("Failed to copy jsonschema_file template");

    // Upload to Gitea
    env.upload_template(repo_name, template_dir.path())
        .expect("Failed to upload template to Gitea");

    // Run baker
    let work_dir = TempDir::new().expect("Failed to create work dir");
    let output_dir = work_dir.path().join("output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let args = Args {
        template: clone_url,
        output_dir: output_dir.clone(),
        force: true,
        verbose: 2,
        answers: None,
        answers_file: None,
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };

    run(args).expect("Baker run failed");

    // Verify output
    let expected_dir = Path::new(&manifest_dir).join("tests/expected/jsonschema_file");
    let output_config = output_dir.join("config.toml");
    assert!(output_config.exists(), "config.toml should be generated");

    let output_content = fs::read_to_string(&output_config).expect("Failed to read output config.toml");
    let expected_content = fs::read_to_string(expected_dir.join("config.toml"))
        .expect("Failed to read expected config.toml");

    assert_eq!(
        output_content.trim(),
        expected_content.trim(),
        "Generated config.toml should match expected output"
    );

    assert!(
        !output_dir.join("database.schema.json").exists(),
        "database.schema.json should be ignored by .bakerignore"
    );

    eprintln!("Successfully generated project from jsonschema_file template via Gitea!");
}

#[test]
#[ignore]
fn test_template_with_submodule_schema_file() {
    let env = get_shared_gitea();

    // Create schema repository
    let schema_repo_name = "shared-schemas";
    let _schema_repo_url = env.create_repo(schema_repo_name).expect("Failed to create schema repository");

    // Add schema file to schema repo
    let schema_content = r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["engine", "host", "port"],
  "properties": {
    "engine": { "type": "string", "enum": ["postgres", "mysql", "sqlite"] },
    "host": { "type": "string" },
    "port": { "type": "integer", "minimum": 1, "maximum": 65535 },
    "ssl": { "type": "boolean" }
  }
}"#;
    env.create_file(schema_repo_name, "database.schema.json", schema_content, "Add schema", "main")
        .expect("Failed to create schema file");

    // Create main template repository
    let main_repo_name = "template-with-submodule";
    let _main_repo_url = env.create_repo(main_repo_name).expect("Failed to create main repository");
    let main_clone_url = env.clone_url_with_auth(main_repo_name);

    // Add baker.yaml to main repo
    let baker_yaml = r#"schemaVersion: v1

questions:
  database_config:
    type: json
    help: Configure your database settings
    schema_file: schemas/database.schema.json
    default: |
      {
        "engine": "postgres",
        "host": "localhost",
        "port": 5432,
        "ssl": false
      }
"#;
    env.create_file(main_repo_name, "baker.yaml", baker_yaml, "Add baker.yaml", "main")
        .expect("Failed to create baker.yaml");

    // Add template file
    let template_content = r#"[database]
engine = "{{ database_config.engine }}"
host = "{{ database_config.host }}"
port = {{ database_config.port }}
ssl = {{ database_config.ssl | lower }}
"#;
    env.create_file(main_repo_name, "config.toml.baker.j2", template_content, "Add template", "main")
        .expect("Failed to create template file");

    // Add .bakerignore
    env.create_file(main_repo_name, ".bakerignore", "schemas/\n", "Add bakerignore", "main")
        .expect("Failed to create .bakerignore");

    // Add the schema file directly (simulating submodule content)
    // In a real submodule scenario, we'd need the schema file accessible
    env.create_file(main_repo_name, "schemas/database.schema.json", schema_content, "Add schema submodule content", "main")
        .expect("Failed to add schema content");

    // Run baker
    let work_dir = TempDir::new().expect("Failed to create work dir");
    let output_dir = work_dir.path().join("output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let args = Args {
        template: main_clone_url,
        output_dir: output_dir.clone(),
        force: true,
        verbose: 2,
        answers: None,
        answers_file: None,
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };

    run(args).expect("Baker run failed - schema_file should be accessible");

    let output_config = output_dir.join("config.toml");
    assert!(output_config.exists(), "config.toml should be generated");

    let output_content = fs::read_to_string(&output_config).expect("Failed to read output config.toml");

    assert!(output_content.contains("engine = \"postgres\""), "Should contain postgres engine");
    assert!(output_content.contains("host = \"localhost\""), "Should contain localhost host");
    assert!(output_content.contains("port = 5432"), "Should contain port 5432");

    eprintln!("Successfully generated project from template with schema_file via Gitea!");
}

#[test]
#[ignore]
fn test_submodule_schema_file_initialization() {
    let env = get_shared_gitea();

    // Create schema repository
    let schema_repo_name = "common-templates-schema";
    let _schema_repo_url = env.create_repo(schema_repo_name).expect("Failed to create schema repository");

    let schema_content = r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "attributes": { "type": "object" }
  }
}"#;
    env.create_file(schema_repo_name, "strapi.schema.json", schema_content, "Add schema", "main")
        .expect("Failed to create schema file");

    // Create main template repository
    let main_repo_name = "template-with-schema-submodule";
    let _main_repo_url = env.create_repo(main_repo_name).expect("Failed to create main repository");
    let main_clone_url = env.clone_url_with_auth(main_repo_name);

    // Add baker.yaml referencing schema in templates/ path
    let baker_yaml = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name
    default: "my_app"

  entities:
    type: json
    help: Configure your entities
    schema_file: "templates/strapi.schema.json"
    default: |
      {}
"#;
    env.create_file(main_repo_name, "baker.yaml", baker_yaml, "Add baker.yaml", "main")
        .expect("Failed to create baker.yaml");

    // Add template file
    let template_content = r#"# {{ project_name }}
Entities count: {{ entities | length }}
"#;
    env.create_file(main_repo_name, "README.md.baker.j2", template_content, "Add template", "main")
        .expect("Failed to create template file");

    // Add schema file in templates/ directory (simulating submodule content)
    env.create_file(main_repo_name, "templates/strapi.schema.json", schema_content, "Add schema in templates", "main")
        .expect("Failed to add schema in templates");

    // Run baker with answers
    let work_dir = TempDir::new().expect("Failed to create work dir");
    let output_dir = work_dir.path().join("output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let answers_file = work_dir.path().join("answers.json");
    let answers_content = r#"{"project_name": "test_app", "entities": {"User": {"name": "User"}}}"#;
    fs::write(&answers_file, answers_content).expect("Failed to write answers file");

    let args = Args {
        template: main_clone_url,
        output_dir: output_dir.clone(),
        force: true,
        verbose: 2,
        answers: None,
        answers_file: Some(answers_file),
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };

    let result = run(args);

    assert!(
        result.is_ok(),
        "Baker should succeed with schema_file. Error: {:?}",
        result.err()
    );

    let output_readme = output_dir.join("README.md");
    assert!(output_readme.exists(), "README.md should be generated");

    let output_content = fs::read_to_string(&output_readme).expect("Failed to read output README.md");

    assert!(
        output_content.contains("test_app"),
        "Output should contain project name. Got: {}",
        output_content
    );

    assert!(
        output_content.contains("Entities count: 1"),
        "Output should contain entities data. Got: {}",
        output_content
    );

    eprintln!("Successfully verified schema_file validation works!");
}
