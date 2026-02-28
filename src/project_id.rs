//! Project identification for home directory storage
//!
//! Each project gets a unique ID based on its git configuration,
//! allowing ~/.promptex/projects/<id>/ to be isolated per project.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
thread_local! {
    static PROMPTEX_HOME_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        std::cell::RefCell::new(None);
}

/// RAII guard that clears the test home override on drop.
/// Ensures cleanup even when a test panics and unwinds.
#[cfg(test)]
pub(crate) struct TestHomeGuard;

#[cfg(test)]
impl Drop for TestHomeGuard {
    fn drop(&mut self) {
        PROMPTEX_HOME_OVERRIDE.with(|o| *o.borrow_mut() = None);
    }
}

/// Redirect all project dir lookups to `path` for the duration of the returned guard.
#[cfg(test)]
pub(crate) fn set_test_home(path: &Path) -> TestHomeGuard {
    PROMPTEX_HOME_OVERRIDE.with(|o| *o.borrow_mut() = Some(path.to_path_buf()));
    TestHomeGuard
}

/// Get unique project identifier for current directory
///
/// Priority:
/// 1. Git remote origin URL (stable across clones)
/// 2. Error if not in git repo (non-git projects unsupported for now)
pub fn get_project_id(cwd: &Path) -> Result<String> {
    // Edge cases to consider:
    // - Multiple remotes (use 'origin' by default, or first remote if no origin)
    // - Non-git directories (return error for now)

    // Get remote origin URL
    let repo_url = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .output()
        .context("Failed to run git command")?;
    
    if !repo_url.status.success() {
        return Err(anyhow::anyhow!("Not a git repository"));
    }

    // Normalize URL
    let url = String::from_utf8(repo_url.stdout).context("Failed to parse git output")?;
    let url = url.trim();
    let url = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .unwrap_or(url);
    let url = url
        .strip_suffix(".git")
        .unwrap_or(url);
    Ok(url.replace("/", "-"))
}

/// Get the project directory in ~/.promptex/projects/<id>/
pub fn get_project_dir(project_id: &str) -> Result<PathBuf> {
    #[cfg(test)]
    {
        let maybe_override = PROMPTEX_HOME_OVERRIDE.with(|o| o.borrow().clone());
        if let Some(base) = maybe_override {
            return Ok(base.join("projects").join(project_id));
        }
    }
    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home.join(".promptex").join("projects").join(project_id))
}

/// Ensure project directory exists, create if missing
pub fn ensure_project_dir(project_id: &str) -> Result<PathBuf> {
    let project_dir = get_project_dir(project_id)?;

    if !project_dir.exists() {
        std::fs::create_dir_all(&project_dir)
            .context("Failed to create project directory")?;
    }

    Ok(project_dir)
}

/// Get path to journal.jsonl for a project
pub fn get_journal_path(project_id: &str) -> Result<PathBuf> {
    let project_dir = get_project_dir(project_id)?;
    Ok(project_dir.join("journal.jsonl"))
}

/// Get path to metadata.json for a project
pub fn get_metadata_path(project_id: &str) -> Result<PathBuf> {
    let project_dir = get_project_dir(project_id)?;
    Ok(project_dir.join("metadata.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_dir_path() {
        let project_id = "github-com-user-repo-abc123";
        let dir = get_project_dir(project_id).unwrap();

        assert!(dir.to_string_lossy().contains(".promptex/projects"));
        assert!(dir.to_string_lossy().ends_with("github-com-user-repo-abc123"));
    }

    #[test]
    fn test_journal_path() {
        let project_id = "test-project";
        let path = get_journal_path(project_id).unwrap();

        assert!(path.to_string_lossy().ends_with("journal.jsonl"));
    }
}
