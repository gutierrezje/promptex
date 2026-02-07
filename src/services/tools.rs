use anyhow::Result;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    Python,
    TypeScript,
    JavaScript,
    Rust,
    Go,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintOutput {
    pub tool: String,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Detect project type by checking for characteristic files
pub fn detect_project_type(repo_path: &Path) -> ProjectType {
    // TODO: Implement project type detection
    todo!("Detect project type")
}

/// Run appropriate linter for the project type
pub fn run_linter(repo_path: &Path, project_type: ProjectType) -> Result<Option<LintOutput>> {
    // TODO: Implement linter execution
    todo!("Run linter")
}

/// Discover test files for the project
pub fn discover_tests(repo_path: &Path, project_type: ProjectType) -> Result<Vec<String>> {
    // TODO: Implement test discovery
    todo!("Discover tests")
}

// TODO(human): Add local signal collectors (CI config, recent commits, TODO/FIXME)
