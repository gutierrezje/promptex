//! Project identification for home directory storage
//!
//! Each project gets a unique ID based on its git configuration,
//! allowing ~/.promptex/projects/<id>/ to be isolated per project.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get unique project identifier for current directory
///
/// Priority:
/// 1. Git remote origin URL (stable across clones)
/// 2. Error if not in git repo (non-git projects unsupported for now)
pub fn get_project_id(cwd: &Path) -> Result<String> {
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

    let url = String::from_utf8(repo_url.stdout).context("Failed to parse git output")?;
    let url = url.trim();
    let url = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .unwrap_or(url);
    let url = url.strip_suffix(".git").unwrap_or(url);
    Ok(url.replace("/", "-"))
}

/// Get the project directory in ~/.promptex/projects/<id>/
pub fn get_project_dir(project_id: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home.join(".promptex").join("projects").join(project_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_dir_path() {
        let project_id = "github-com-user-repo-abc123";
        let dir = get_project_dir(project_id).unwrap();

        assert!(dir.to_string_lossy().contains(".promptex/projects"));
        assert!(dir
            .to_string_lossy()
            .ends_with("github-com-user-repo-abc123"));
    }
}
