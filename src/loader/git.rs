use crate::{
    error::{Error, Result},
    prompt::confirm,
};
use gix::progress::Discard;
use std::fs;
use std::path::PathBuf;
use url::Url;

use crate::loader::interface::TemplateLoader;

/// Loader for templates from git repositories.
pub struct GitLoader<S: AsRef<str>> {
    repo: S,
    skip_overwrite_check: bool,
}

impl<S: AsRef<str>> GitLoader<S> {
    /// Creates a new GitLoader instance.
    pub fn new(repo: S, skip_overwrite_check: bool) -> Self {
        Self { repo, skip_overwrite_check }
    }

    /// Extracts repository name from various git URL formats.
    ///
    /// Supports:
    /// - HTTPS: https://github.com/user/repo.git -> repo
    /// - SSH: git@github.com:user/repo.git -> repo
    /// - SSH without .git: git@github.com:user/repo -> repo
    pub fn extract_repo_name(repo_url: &str) -> String {
        if repo_url.is_empty() {
            return "template".to_string();
        }

        // Handle SSH format: git@host:user/repo or user@host:user/repo
        if repo_url.contains('@') && repo_url.contains(':') && !repo_url.contains("://") {
            if let Some(colon_pos) = repo_url.rfind(':') {
                let path_part = &repo_url[colon_pos + 1..];
                if !path_part.is_empty() {
                    return path_part
                        .split('/')
                        .next_back()
                        .unwrap_or("template")
                        .trim_end_matches(".git")
                        .to_string();
                }
            }
        }

        // Handle standard URLs (HTTPS, git://, etc.)
        let result =
            repo_url.split('/').next_back().unwrap_or("").trim_end_matches(".git");

        if result.is_empty() || result.contains('@') || result.contains(':') {
            "template".to_string()
        } else {
            result.to_string()
        }
    }

    /// Determines if a string represents a git repository URL.
    ///
    /// Supports:
    /// - HTTPS URLs: https://github.com/user/repo
    /// - Git URLs: git://github.com/user/repo
    /// - SSH URLs: git@github.com:user/repo
    /// - SSH URLs with explicit protocol: ssh://git@github.com/user/repo
    pub fn is_git_url(s: &str) -> bool {
        // Try to parse as standard URL first
        if let Ok(url) = Url::parse(s) {
            return matches!(url.scheme(), "http" | "https" | "git" | "ssh");
        }

        // Check for SSH format: git@host:path or user@host:path
        if s.contains('@') && s.contains(':') && !s.contains("://") {
            // Simple heuristic: if it contains @ and : but not ://, it's likely SSH format
            // Also check that the part after @ and before : looks like a hostname
            if let Some(at_pos) = s.find('@') {
                if let Some(colon_pos) = s.rfind(':') {
                    if colon_pos > at_pos {
                        let user_part = &s[..at_pos];
                        let host_part = &s[at_pos + 1..colon_pos];
                        let path_part = &s[colon_pos + 1..];

                        // More strict validation:
                        // - user part should look like a username (git, or valid username)
                        // - host should look like a hostname (contains . or known git hosts)
                        // - path should look like a repository path (contains /)
                        return !user_part.is_empty()
                            && !host_part.is_empty()
                            && !path_part.is_empty()
                            && (host_part.contains('.')
                                || host_part == "github.com"
                                || host_part == "gitlab.com"
                                || host_part == "bitbucket.org")
                            && path_part.contains('/');
                    }
                }
            }
        }

        false
    }

    /// Recursively initializes and updates all submodules in a repository.
    fn init_submodules(&self, repo: &gix::Repository) -> Result<()> {
        // Get the list of submodules from the repository
        let submodules = match repo.submodules() {
            Ok(Some(platform)) => platform,
            Ok(None) => return Ok(()),
            Err(e) => {
                log::debug!("No submodules found or error reading submodules: {}", e);
                return Ok(());
            }
        };

        for submodule in submodules {
            let name = submodule.name().to_string();
            log::debug!("Initializing submodule: {}", name);

            let submodule_path = match submodule.path() {
                Ok(p) => PathBuf::from(p.to_string()),
                Err(e) => {
                    log::warn!("Failed to get submodule path for {}: {}", name, e);
                    continue;
                }
            };

            let submodule_url = match submodule.url() {
                Ok(url) => url.to_bstring().to_string(),
                Err(e) => {
                    log::warn!("Failed to get URL for submodule {}: {}", name, e);
                    continue;
                }
            };

            let full_path = repo
                .workdir()
                .ok_or_else(|| Error::GitError("No working directory".to_string()))?
                .join(&submodule_path);

            log::debug!(
                "Cloning submodule {} from {} to {:?}",
                name,
                submodule_url,
                full_path
            );

            // Clone the submodule
            if !full_path.exists()
                || full_path.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true)
            {
                match gix::clone::PrepareFetch::new(
                    gix::url::parse(submodule_url.as_str().into()).map_err(|e| {
                        Error::GitError(format!("Invalid submodule URL: {}", e))
                    })?,
                    &full_path,
                    gix::create::Kind::WithWorktree,
                    gix::create::Options::default(),
                    gix::open::Options::isolated(),
                ) {
                    Ok(mut clone) => {
                        let (mut checkout, _outcome) = clone
                            .fetch_then_checkout(Discard, &gix::interrupt::IS_INTERRUPTED)
                            .map_err(|e| {
                                Error::GitError(format!(
                                    "Failed to fetch submodule {}: {}",
                                    name, e
                                ))
                            })?;

                        let (sub_repo, _outcome) = checkout
                            .main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED)
                            .map_err(|e| {
                                Error::GitError(format!(
                                    "Failed to checkout submodule {}: {}",
                                    name, e
                                ))
                            })?;

                        // Recursively init submodules of this submodule
                        let sub_repo_local = sub_repo.into_sync().to_thread_local();
                        self.init_submodules(&sub_repo_local)?;
                    }
                    Err(e) => {
                        log::warn!("Failed to clone submodule {}: {}", name, e);
                    }
                }
            }
        }
        Ok(())
    }
}

impl<S: AsRef<str>> TemplateLoader for GitLoader<S> {
    /// Loads a template by cloning a git repository.
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Path to the cloned repository
    fn load(&self) -> Result<PathBuf> {
        let repo_url = self.repo.as_ref();

        log::debug!("Cloning repository '{repo_url}'");

        let repo_name = Self::extract_repo_name(repo_url);
        let clone_path = PathBuf::from(&repo_name);

        if clone_path.exists() {
            let response = confirm(
                self.skip_overwrite_check,
                format!("Directory '{repo_name}' already exists. Replace it?"),
            )?;
            if response {
                fs::remove_dir_all(&clone_path)?;
            } else {
                log::debug!("Using existing directory '{}'", clone_path.display());
                return Ok(clone_path);
            }
        }

        log::debug!("Cloning to '{}'", clone_path.display());

        // Parse the URL
        let url = gix::url::parse(repo_url.into())
            .map_err(|e| Error::GitError(format!("Invalid repository URL: {}", e)))?;

        // Prepare the clone operation
        let mut clone = gix::clone::PrepareFetch::new(
            url,
            &clone_path,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            gix::open::Options::isolated(),
        )
        .map_err(|e| Error::GitError(format!("Failed to prepare clone: {}", e)))?;

        // Fetch and checkout
        let (mut checkout, _outcome) = clone
            .fetch_then_checkout(Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| Error::GitError(format!("Failed to fetch repository: {}", e)))?;

        let (repo, _outcome) =
            checkout.main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED).map_err(
                |e| Error::GitError(format!("Failed to checkout repository: {}", e)),
            )?;

        // Initialize and update submodules recursively
        let repo_local = repo.into_sync().to_thread_local();
        self.init_submodules(&repo_local)?;

        Ok(clone_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url_http() {
        assert!(GitLoader::<&str>::is_git_url("http://localhost:3000/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("http://localhost:3000/user/repo.git"));
        assert!(GitLoader::<&str>::is_git_url("http://192.168.1.1/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("http://gitea.local/user/repo.git"));
    }

    #[test]
    fn test_is_git_url_https() {
        assert!(GitLoader::<&str>::is_git_url("https://github.com/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("https://github.com/user/repo.git"));
        assert!(GitLoader::<&str>::is_git_url("https://gitlab.com/user/repo"));
    }

    #[test]
    fn test_is_git_url_ssh() {
        assert!(GitLoader::<&str>::is_git_url("git@github.com:user/repo"));
        assert!(GitLoader::<&str>::is_git_url("git@github.com:user/repo.git"));
        assert!(GitLoader::<&str>::is_git_url("git@gitlab.com:user/repo"));
        assert!(GitLoader::<&str>::is_git_url("user@bitbucket.org:user/repo"));
    }

    #[test]
    fn test_is_git_url_git_protocol() {
        assert!(GitLoader::<&str>::is_git_url("git://github.com/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("ssh://git@github.com/user/repo"));
    }

    #[test]
    fn test_is_git_url_local_paths() {
        assert!(!GitLoader::<&str>::is_git_url("/path/to/local/template"));
        assert!(!GitLoader::<&str>::is_git_url("./relative/path"));
        assert!(!GitLoader::<&str>::is_git_url("../parent/path"));
        assert!(!GitLoader::<&str>::is_git_url("template"));
        assert!(!GitLoader::<&str>::is_git_url("C:\\Windows\\Path"));
    }

    #[test]
    fn test_is_git_url_invalid_ssh() {
        // Should not match SSH-like strings that aren't actually git URLs
        assert!(!GitLoader::<&str>::is_git_url("user@localhost:file.txt"));
        assert!(!GitLoader::<&str>::is_git_url("name@email.com:something"));
        assert!(!GitLoader::<&str>::is_git_url("user@host"));
        assert!(!GitLoader::<&str>::is_git_url("@host:path"));
    }

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            GitLoader::<String>::extract_repo_name("https://github.com/user/repo"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("https://github.com/user/repo.git"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name(
                "https://gitlab.com/group/subgroup/repo.git"
            ),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            GitLoader::<String>::extract_repo_name("git@github.com:user/repo"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("git@github.com:user/repo.git"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("user@gitlab.com:group/repo"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_edge_cases() {
        assert_eq!(GitLoader::<String>::extract_repo_name("invalid-url"), "invalid-url");
        assert_eq!(GitLoader::<String>::extract_repo_name(""), "template");
        assert_eq!(GitLoader::<String>::extract_repo_name("git@host:"), "template");
    }
}
