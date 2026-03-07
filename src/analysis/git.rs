//! Git operations for reading repository state.
//!
//! All functions shell out to git rather than using a library, keeping the
//! dependency footprint minimal and behaviour consistent with what the user
//! already has installed.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use std::process::Command;

/// A git commit with its changed file list pre-loaded.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Abbreviated 7-character SHA.
    pub short_hash: String,
    /// First line of the commit message.
    pub message: String,
    /// Author date in UTC.
    pub timestamp: DateTime<Utc>,
    /// Files changed in this commit.
    pub files: Vec<String>,
}

// ── Branch helpers ────────────────────────────────────────────────────────────

/// Return the name of the currently checked-out branch.
///
/// Errors if the repo is in detached HEAD state.
pub fn current_branch() -> Result<String> {
    let out = git(&["branch", "--show-current"])?;
    let branch = out.trim().to_string();
    if branch.is_empty() {
        bail!("Detached HEAD — not on a named branch");
    }
    Ok(branch)
}

/// Return true if `branch` is a mainline branch (main, master, develop, trunk).
pub fn is_mainline_branch(branch: &str) -> bool {
    matches!(
        branch,
        "main" | "master" | "develop" | "trunk" | "development"
    )
}

/// Find the first mainline branch that exists in this repo.
pub fn find_mainline_branch() -> Result<String> {
    for candidate in ["main", "master", "develop", "trunk"] {
        let ok = Command::new("git")
            .args(["rev-parse", "--verify", "--quiet", candidate])
            .output()
            .context("Failed to run git rev-parse")?
            .status
            .success();
        if ok {
            return Ok(candidate.to_string());
        }
    }
    bail!("Could not find a mainline branch (main, master, develop, trunk)");
}

/// Return the commit hash where the current branch diverged from mainline.
///
/// Prefers a remote-tracking ref (`upstream/<mainline>`, then `origin/<mainline>`)
/// over the local branch, so fork workflows get the correct diverge point even
/// when the local mainline is stale.
pub fn branch_diverge_point() -> Result<String> {
    let mainline = find_mainline_branch()?;
    let merge_base_ref = resolve_merge_base_ref(&mainline);
    let out = git(&["merge-base", "HEAD", &merge_base_ref])
        .with_context(|| format!("Could not compute diverge point from '{merge_base_ref}'"))?;
    Ok(out.trim().to_string())
}

// ── Working-tree state ────────────────────────────────────────────────────────

/// Return true if there are any staged or unstaged changes.
pub fn has_uncommitted_changes() -> Result<bool> {
    let out = git(&["status", "--porcelain"])?;
    Ok(!out.trim().is_empty())
}

/// List files that have uncommitted changes (staged or unstaged, including untracked).
pub fn uncommitted_files() -> Result<Vec<String>> {
    let out = git(&["status", "--porcelain"])?;
    let files = out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            // Format: "XY filename" or "XY old -> new" for renames
            let name = l[3..].trim();
            // For renames git outputs "old -> new"; take the new path
            if let Some((_, new)) = name.split_once(" -> ") {
                new.to_string()
            } else {
                name.to_string()
            }
        })
        .collect();
    Ok(files)
}

// ── Commit loading ────────────────────────────────────────────────────────────

/// Load all commits reachable from HEAD but not from `since_hash` (exclusive).
///
/// Returned in chronological order (oldest first).
pub fn commits_since(since_hash: &str) -> Result<Vec<Commit>> {
    let range = format!("{since_hash}..HEAD");
    load_commits(&["log", &range, "--format=%H|%h|%aI|%s", "--reverse"])
}

/// Load the last `n` commits ending at HEAD.
///
/// Returned in chronological order (oldest first).
pub fn last_n_commits(n: usize) -> Result<Vec<Commit>> {
    let n_str = format!("-{n}");
    load_commits(&["log", &n_str, "--format=%H|%h|%aI|%s", "--reverse"])
}

/// Load all commits authored after `since` (exclusive lower bound by timestamp).
///
/// Returned in chronological order (oldest first).
pub fn commits_since_time(since: DateTime<Utc>) -> Result<Vec<Commit>> {
    let since_str = since.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    load_commits(&[
        "log",
        &format!("--after={since_str}"),
        "--format=%H|%h|%aI|%s",
        "--reverse",
    ])
}

// ── Internals ─────────────────────────────────────────────────────────────────

/// Run `git <args>` and return stdout as a String. Errors on non-zero exit.
fn git(args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    String::from_utf8(out.stdout).context("git output is not valid UTF-8")
}

// Prefer upstream/<mainline> → origin/<mainline> → local <mainline>.
// Only checks refs already fetched locally — no git fetch is performed.
fn resolve_merge_base_ref(mainline: &str) -> String {
    for prefix in ["upstream", "origin"] {
        let candidate = format!("{prefix}/{mainline}");
        let exists = Command::new("git")
            .args(["rev-parse", "--verify", "--quiet", &candidate])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if exists {
            return candidate;
        }
    }
    mainline.to_string()
}

fn load_commits(log_args: &[&str]) -> Result<Vec<Commit>> {
    let raw = git(log_args)?;
    let mut commits = Vec::new();

    for line in raw.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() < 4 {
            continue;
        }

        let hash = parts[0];
        let short_hash = parts[1].to_string();
        let timestamp = DateTime::parse_from_rfc3339(parts[2])
            .with_context(|| format!("Failed to parse commit timestamp: {}", parts[2]))?
            .with_timezone(&Utc);
        let message = parts[3].to_string();
        let files = files_in_commit(hash)?;

        commits.push(Commit {
            short_hash,
            message,
            timestamp,
            files,
        });
    }

    Ok(commits)
}

/// List files changed in a single commit.
fn files_in_commit(hash: &str) -> Result<Vec<String>> {
    let raw = git(&["diff-tree", "--no-commit-id", "-r", "--name-only", hash])?;
    Ok(raw
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mainline_branch() {
        assert!(is_mainline_branch("main"));
        assert!(is_mainline_branch("master"));
        assert!(is_mainline_branch("develop"));
        assert!(is_mainline_branch("trunk"));
        assert!(!is_mainline_branch("feature/auth"));
        assert!(!is_mainline_branch("fix/bug-123"));
        assert!(!is_mainline_branch(""));
    }

    #[test]
    fn test_current_branch_returns_string() {
        // This repo is a git repo, so current_branch() should succeed.
        let branch = current_branch().unwrap();
        assert!(!branch.is_empty());
    }

    #[test]
    fn test_last_n_commits_returns_commits() {
        let commits = last_n_commits(3).unwrap();
        // We have at least one commit in this repo.
        assert!(!commits.is_empty());
        assert!(commits.len() <= 3);
        // Each commit should have a non-empty hash and message.
        for c in &commits {
            assert_eq!(c.short_hash.len(), 7);
            assert!(!c.message.is_empty());
        }
    }

    #[test]
    fn test_has_uncommitted_changes_does_not_panic() {
        // Just verify it runs without error — actual value depends on repo state.
        has_uncommitted_changes().unwrap();
    }

    #[test]
    fn test_resolve_merge_base_ref_prefers_origin_over_local() {
        // This repo has origin/main but no upstream remote.
        // Should prefer "origin/main" over the bare local branch name.
        let result = resolve_merge_base_ref("main");
        assert_eq!(result, "origin/main");
    }

    #[test]
    fn test_resolve_merge_base_ref_falls_back_to_local() {
        // No remote has a branch with this name — should return the name unchanged.
        let result = resolve_merge_base_ref("nonexistentxyz123");
        assert_eq!(result, "nonexistentxyz123");
    }
}
