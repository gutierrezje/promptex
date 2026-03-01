//! Extraction scope — what range of work to extract prompts for.
//!
//! Scope is determined either from explicit CLI flags or by applying smart
//! defaults based on the current git state.

use super::git;
use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};

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
    /// All commits authored since a relative time offset (e.g. 2h, 1d).
    SinceTime(DateTime<Utc>),
}

/// CLI flags that control scope selection, passed in from `commands::extract`.
pub struct ScopeFlags {
    pub uncommitted: bool,
    pub commits: Option<usize>,
    pub since_commit: Option<String>,
    pub branch_lifetime: bool,
    /// Duration string like "2h", "30m", "1d", "3w".
    pub since_duration: Option<String>,
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
    if let Some(dur) = &flags.since_duration {
        let since = parse_duration_str(dur)?;
        return Ok(ExtractionScope::SinceTime(since));
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
        return Ok(ExtractionScope::BranchLifetime {
            branch,
            since_commit,
        });
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
        Ok(ExtractionScope::BranchLifetime {
            branch,
            since_commit,
        })
    }
}

/// Parse a human duration string into a `DateTime<Utc>` representing `now - duration`.
///
/// Supported units: `m` (minutes), `h` (hours), `d` (days), `w` (weeks).
/// Example: `"2h"` → two hours ago, `"30m"` → thirty minutes ago.
fn parse_duration_str(s: &str) -> Result<DateTime<Utc>> {
    if s.is_empty() {
        bail!("Duration string is empty — expected format like '2h', '30m', '1d', '3w'");
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let n: i64 = num_str.parse().map_err(|_| {
        anyhow::anyhow!("Invalid duration '{s}' — expected format like '2h', '30m', '1d', '3w'")
    })?;

    if n <= 0 {
        bail!("Duration must be positive, got '{s}'");
    }

    let duration = match unit {
        "m" => Duration::minutes(n),
        "h" => Duration::hours(n),
        "d" => Duration::days(n),
        "w" => Duration::weeks(n),
        other => bail!("Unknown duration unit '{other}' in '{s}' — use m, h, d, or w"),
    };

    Ok(Utc::now() - duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_uncommitted_flag_wins() {
        let flags = ScopeFlags {
            uncommitted: true,
            commits: Some(5), // would be Commits(5) without the uncommitted flag
            since_commit: None,
            branch_lifetime: false,
            since_duration: None,
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
            since_duration: None,
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
            since_duration: None,
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
            since_duration: None,
        };
        determine_scope(&flags).unwrap();
    }

    // ── parse_duration_str ────────────────────────────────────────────────────

    #[test]
    fn test_parse_duration_minutes() {
        let before = Utc::now() - Duration::minutes(30);
        let result = parse_duration_str("30m").unwrap();
        let after = Utc::now() - Duration::minutes(30);
        // Result should be between before and after (within a second of 30m ago)
        assert!(result >= before - Duration::seconds(1));
        assert!(result <= after + Duration::seconds(1));
    }

    #[test]
    fn test_parse_duration_hours() {
        let before = Utc::now() - Duration::hours(2);
        let result = parse_duration_str("2h").unwrap();
        let after = Utc::now() - Duration::hours(2);
        assert!(result >= before - Duration::seconds(1));
        assert!(result <= after + Duration::seconds(1));
    }

    #[test]
    fn test_parse_duration_days() {
        let before = Utc::now() - Duration::days(1);
        let result = parse_duration_str("1d").unwrap();
        let after = Utc::now() - Duration::days(1);
        assert!(result >= before - Duration::seconds(1));
        assert!(result <= after + Duration::seconds(1));
    }

    #[test]
    fn test_parse_duration_weeks() {
        let before = Utc::now() - Duration::weeks(3);
        let result = parse_duration_str("3w").unwrap();
        let after = Utc::now() - Duration::weeks(3);
        assert!(result >= before - Duration::seconds(1));
        assert!(result <= after + Duration::seconds(1));
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        let err = parse_duration_str("5x").unwrap_err();
        assert!(err.to_string().contains("Unknown duration unit"));
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        let err = parse_duration_str("abch").unwrap_err();
        assert!(err.to_string().contains("Invalid duration"));
    }

    #[test]
    fn test_parse_duration_empty() {
        let err = parse_duration_str("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_explicit_since_duration_flag() {
        let flags = ScopeFlags {
            uncommitted: false,
            commits: None,
            since_commit: None,
            branch_lifetime: false,
            since_duration: Some("1h".to_string()),
        };
        let scope = determine_scope(&flags).unwrap();
        assert!(matches!(scope, ExtractionScope::SinceTime(_)));
    }

    #[test]
    fn test_since_duration_wins_over_since_commit() {
        // --since has higher priority than --since-commit per the plan
        let flags = ScopeFlags {
            uncommitted: false,
            commits: None,
            since_commit: Some("abc123".to_string()),
            branch_lifetime: false,
            since_duration: Some("2h".to_string()),
        };
        let scope = determine_scope(&flags).unwrap();
        assert!(matches!(scope, ExtractionScope::SinceTime(_)));
    }
}
