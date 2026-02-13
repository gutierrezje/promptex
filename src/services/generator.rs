use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use serde::Serialize;

/// Project-scoped context root: <repo>/.issuance
pub fn project_context_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".issuance")
}

/// Issue-scoped context directory: <repo>/.issuance/issues/<issue_number>
pub fn issue_context_dir(repo_root: &Path, issue_number: u64) -> PathBuf {
    project_context_dir(repo_root)
        .join("issues")
        .join(issue_number.to_string())
}

/// Ensure context directories exist and return (project_dir, issue_dir)
pub fn ensure_context_dirs(repo_root: &Path, issue_number: u64) -> Result<(PathBuf, PathBuf)> {
    let project_dir = project_context_dir(repo_root);
    let issue_dir = issue_context_dir(repo_root, issue_number);

    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(&issue_dir)?;

    Ok((project_dir, issue_dir))
}

/// Generate all issue-scoped context pack files in .issuance/issues/<issue_number>/
pub fn generate_context_pack(
    output_dir: &Path,
    issue_data: &impl Serialize,
    codemap_data: &impl Serialize,
    signals_data: &impl Serialize,
    rules_data: &impl Serialize,
    handoff_data: &impl Serialize,
) -> Result<()> {
    // TODO: Implement template rendering and file generation
    todo!("Generate context pack files")
}

/// Generate metadata.json for the issue-scoped session directory
pub fn generate_metadata(
    output_dir: &Path,
    issue_url: &str,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<()> {
    // TODO: Implement metadata generation
    todo!("Generate metadata.json")
}
