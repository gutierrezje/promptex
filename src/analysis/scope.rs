//! Extraction scope — what range of work to extract prompts for.
//!
//! Scope is determined either from explicit CLI flags or by applying smart
//! defaults based on the current git state.

use super::git;
use anyhow::Result;

/// The range of work that `pmtx extract` should cover.
#[derive(Debug, Clone)]
pub enum ExtractionScope {
    /// All commits on a feature branch since it diverged from mainline.
    BranchLifetime {
        branch: String,
        /// Commit hash where the branch diverged (exclusive lower bound).
        since_commit: String,
    },
    /// The last N commits on the current branch.
    LastNCommits(usize),
    /// All commits since a specific hash (exclusive).
    SinceCommit(String),
    /// Only uncommitted changes (staged + unstaged).
    Uncommitted,
}

/// CLI flags that control scope selection, passed in from `commands::extract`.
pub struct ScopeFlags {
    pub uncommitted: bool,
    pub commits: Option<usize>,
    pub since_commit: Option<String>,
    pub branch_lifetime: bool,
}

/// Determine the extraction scope.
///
/// Explicit flags take priority in this order:
/// 1. `--uncommitted`
/// 2. `--since-commit <HASH>`
/// 3. `--commits <N>`
/// 4. `--branch-lifetime`
///
/// If no flags are set, smart defaults apply:
/// - Feature branch → `BranchLifetime` (since diverge from mainline)
/// - Mainline with uncommitted changes → `Uncommitted`
/// - Mainline with no uncommitted changes → `LastNCommits(1)`
pub fn determine_scope(flags: &ScopeFlags) -> Result<ExtractionScope> {
    // Explicit flags — checked in priority order
    if flags.uncommitted {
        return Ok(ExtractionScope::Uncommitted);
    }
    if let Some(hash) = &flags.since_commit {
        return Ok(ExtractionScope::SinceCommit(hash.clone()));
    }
    if let Some(n) = flags.commits {
        return Ok(ExtractionScope::LastNCommits(n));
    }
    if flags.branch_lifetime {
        let branch = git::current_branch()?;
        let since_commit = git::branch_diverge_point()?;
        return Ok(ExtractionScope::BranchLifetime { branch, since_commit });
    }

    // Smart defaults
    let branch = git::current_branch()?;

    if git::is_mainline_branch(&branch) {
        if git::has_uncommitted_changes()? {
            Ok(ExtractionScope::Uncommitted)
        } else {
            Ok(ExtractionScope::LastNCommits(1))
        }
    } else {
        let since_commit = git::branch_diverge_point()?;
        Ok(ExtractionScope::BranchLifetime { branch, since_commit })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_uncommitted_flag_wins() {
        let flags = ScopeFlags {
            uncommitted: true,
            commits: Some(5),       // would be Commits(5) without the uncommitted flag
            since_commit: None,
            branch_lifetime: false,
        };
        let scope = determine_scope(&flags).unwrap();
        assert!(matches!(scope, ExtractionScope::Uncommitted));
    }

    #[test]
    fn test_explicit_since_commit_flag() {
        let flags = ScopeFlags {
            uncommitted: false,
            commits: None,
            since_commit: Some("abc123".to_string()),
            branch_lifetime: false,
        };
        let scope = determine_scope(&flags).unwrap();
        assert!(matches!(scope, ExtractionScope::SinceCommit(h) if h == "abc123"));
    }

    #[test]
    fn test_explicit_commits_flag() {
        let flags = ScopeFlags {
            uncommitted: false,
            commits: Some(3),
            since_commit: None,
            branch_lifetime: false,
        };
        let scope = determine_scope(&flags).unwrap();
        assert!(matches!(scope, ExtractionScope::LastNCommits(3)));
    }

    #[test]
    fn test_smart_default_runs_without_error() {
        // Smart default calls into git — just verify it doesn't panic or error
        // in a normal repo context (which the test runner provides).
        let flags = ScopeFlags {
            uncommitted: false,
            commits: None,
            since_commit: None,
            branch_lifetime: false,
        };
        determine_scope(&flags).unwrap();
    }
}
