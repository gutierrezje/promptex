//! Derive the per-project storage key used under `~/.promptex/projects/`.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a project identifier from repository metadata.
///
/// Resolution order:
/// 1. `remote.origin.url`
/// 2. `remote.upstream.url`
/// 3. Any other configured remote URL
/// 4. Deterministic fallback from canonical git root path
pub fn get_project_id(cwd: &Path) -> Result<String> {
    if !is_git_repository(cwd)? {
        return Err(anyhow!("Not a git repository"));
    }

    if let Some(url) = get_remote_url(cwd, "origin")? {
        return Ok(project_id_from_remote(&url));
    }
    if let Some(url) = get_remote_url(cwd, "upstream")? {
        return Ok(project_id_from_remote(&url));
    }

    for remote in list_remotes(cwd)?
        .into_iter()
        .filter(|r| r != "origin" && r != "upstream")
    {
        if let Some(url) = get_remote_url(cwd, &remote)? {
            return Ok(project_id_from_remote(&url));
        }
    }

    let root = git_root(cwd)?;
    Ok(project_id_from_path(&root))
}

fn is_git_repository(cwd: &Path) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .context("Failed to run git command")?;

    Ok(out.status.success())
}

fn git_root(cwd: &Path) -> Result<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .context("Failed to run git command")?;

    if !out.status.success() {
        return Err(anyhow!("Not a git repository"));
    }

    let root = String::from_utf8(out.stdout).context("Failed to parse git output")?;
    Ok(PathBuf::from(root.trim()))
}

fn list_remotes(cwd: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("remote")
        .output()
        .context("Failed to run git command")?;

    if !out.status.success() {
        return Err(anyhow!("Not a git repository"));
    }

    let names = String::from_utf8(out.stdout)
        .context("Failed to parse git output")?
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    Ok(names)
}

fn get_remote_url(cwd: &Path, remote: &str) -> Result<Option<String>> {
    let key = format!("remote.{remote}.url");
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("config")
        .arg("--get")
        .arg(key)
        .output()
        .context("Failed to run git command")?;

    if !out.status.success() {
        return Ok(None);
    }

    let value = String::from_utf8(out.stdout).context("Failed to parse git output")?;
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}

fn project_id_from_remote(url: &str) -> String {
    let trimmed = url.trim().strip_suffix(".git").unwrap_or(url.trim());
    let normalized = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
        .unwrap_or(trimmed);
    sanitize_id(&normalized.replace('/', "-"))
}

fn project_id_from_path(root: &Path) -> String {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let hash = fnv1a64(canonical.to_string_lossy().as_bytes());
    format!("local-{hash:016x}")
}

fn sanitize_id(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_sep = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('-');
            prev_sep = true;
        }
    }

    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Return the storage directory for a project ID.
pub fn get_project_dir(project_id: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home.join(".promptex").join("projects").join(project_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed", args);
    }

    fn init_repo(cwd: &Path) {
        git(&["init", "-q"], cwd);
        git(&["config", "user.email", "test@example.com"], cwd);
        git(&["config", "user.name", "Test User"], cwd);
    }

    #[test]
    fn test_project_dir_path() {
        let project_id = "github-com-user-repo-abc123";
        let dir = get_project_dir(project_id).unwrap();

        assert!(dir.to_string_lossy().contains(".promptex/projects"));
        assert!(dir
            .to_string_lossy()
            .ends_with("github-com-user-repo-abc123"));
    }

    #[test]
    fn test_project_id_prefers_origin_remote() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        git(
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/org/origin-repo.git",
            ],
            dir.path(),
        );
        git(
            &[
                "remote",
                "add",
                "upstream",
                "https://github.com/org/upstream-repo.git",
            ],
            dir.path(),
        );

        let id = get_project_id(dir.path()).unwrap();
        assert_eq!(id, "org-origin-repo");
    }

    #[test]
    fn test_project_id_uses_upstream_when_origin_missing() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        git(
            &[
                "remote",
                "add",
                "upstream",
                "https://github.com/org/upstream-repo.git",
            ],
            dir.path(),
        );

        let id = get_project_id(dir.path()).unwrap();
        assert_eq!(id, "org-upstream-repo");
    }

    #[test]
    fn test_project_id_falls_back_when_no_remotes() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());

        let id = get_project_id(dir.path()).unwrap();
        assert!(id.starts_with("local-"));
        assert_eq!(id.len(), "local-".len() + 16);
    }

    #[test]
    fn test_project_id_errors_for_non_git_directory() {
        let dir = tempdir().unwrap();
        let err = get_project_id(dir.path()).unwrap_err();
        assert!(err.to_string().contains("Not a git repository"));
    }

    #[test]
    fn test_project_id_sanitizes_ssh_remote() {
        let id = project_id_from_remote("git@bitbucket.org:MyTeam/MyRepo.git");
        assert_eq!(id, "git-bitbucket-org-myteam-myrepo");
    }

    #[test]
    fn test_project_id_from_path_is_deterministic() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        let root = git_root(dir.path()).unwrap();

        let a = project_id_from_path(&root);
        let b = project_id_from_path(&root);
        assert_eq!(a, b);
    }
}
