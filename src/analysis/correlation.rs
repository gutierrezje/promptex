//! Correlation — match extracted journal entries to the current git scope.
//!
//! The pipeline is:
//!   1. `build_git_context` — resolve the scope once into files + time window
//!   2. `filter_by_scope`   — keep entries that touch scoped files or fall in the window

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

use crate::analysis::git::{self, Commit};
use crate::analysis::scope::ExtractionScope;
use crate::journal::JournalEntry;

// ── GitContext ─────────────────────────────────────────────────────────────────

/// Pre-resolved git context for an extraction scope.
///
/// Computing this once avoids repeated git calls during filtering. Consumers
/// pass this to [`filter_by_scope`] and also use `since`/`until` to bound the
/// time window they feed to the extractor.
pub struct GitContext {
    /// All files that are in scope, deduplicated.
    ///
    /// For commit-based scopes this is the union of files changed in every
    /// commit. For [`ExtractionScope::Uncommitted`] it is the list of
    /// modified/staged files from `git status`.
    pub scope_files: Vec<String>,

    /// Inclusive lower bound for time-window filtering.
    pub since: DateTime<Utc>,

    /// Inclusive upper bound for time-window filtering (typically now).
    pub until: DateTime<Utc>,

    /// Commits in scope. Empty for [`ExtractionScope::Uncommitted`].
    pub commits: Vec<Commit>,
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Build a [`GitContext`] by resolving the scope against live git state.
///
/// Each scope variant resolves differently:
/// - Commit-based scopes (`BranchLifetime`, `LastNCommits`, `SinceCommit`) load
///   their commits, collect the union of changed files, and use the earliest
///   commit timestamp as `since`.
/// - [`ExtractionScope::Uncommitted`] uses `git status` for files and the HEAD
///   commit timestamp as `since` (everything since the last commit).
pub fn build_git_context(scope: &ExtractionScope) -> Result<GitContext> {
    let until = Utc::now();

    match scope {
        ExtractionScope::BranchLifetime { since_commit, .. } => {
            let commits = git::commits_since(since_commit)?;
            let scope_files = collect_files(&commits);
            let since = earliest_commit_time(&commits).unwrap_or_else(|| until - Duration::days(7));
            Ok(GitContext {
                scope_files,
                since,
                until,
                commits,
            })
        }

        ExtractionScope::LastNCommits(n) => {
            // Load one extra commit as a look-behind anchor so we can bound the
            // window to [anchor.timestamp, scope_commits.latest.timestamp].
            // Prompts that produced a commit happen *before* it, not after, so
            // `until` must be the commit time rather than `now`.
            let all = git::last_n_commits(n + 1)?;
            let (scope_commits, since) = if all.len() > *n {
                // oldest entry is the anchor (just outside scope)
                let anchor_time = all[0].timestamp;
                (all[1..].to_vec(), anchor_time)
            } else {
                // fewer commits than requested — use all, fall back for since
                let fallback =
                    earliest_commit_time(&all).unwrap_or_else(|| until - Duration::days(7));
                (all, fallback)
            };
            let scope_files = collect_files(&scope_commits);
            let commit_until = latest_commit_time(&scope_commits).unwrap_or(until);
            Ok(GitContext {
                scope_files,
                since,
                until: commit_until,
                commits: scope_commits,
            })
        }

        ExtractionScope::SinceCommit(hash) => {
            let commits = git::commits_since(hash)?;
            let scope_files = collect_files(&commits);
            let since = earliest_commit_time(&commits).unwrap_or_else(|| until - Duration::days(7));
            Ok(GitContext {
                scope_files,
                since,
                until,
                commits,
            })
        }

        ExtractionScope::Uncommitted => {
            let scope_files = git::uncommitted_files()?;
            // Lower bound: HEAD commit time — "everything since the last commit".
            let since = git::last_n_commits(1)?
                .into_iter()
                .next()
                .map(|c| c.timestamp)
                .unwrap_or_else(|| until - Duration::hours(24));
            Ok(GitContext {
                scope_files,
                since,
                until,
                commits: Vec::new(),
            })
        }

        ExtractionScope::SinceTime(since) => {
            let commits = git::commits_since_time(*since)?;
            let scope_files = collect_files(&commits);
            Ok(GitContext {
                scope_files,
                since: *since,
                until,
                commits,
            })
        }
    }
}

/// Filter journal entries to those relevant to the given git context.
///
/// An entry is **kept** if either condition is true:
/// 1. It touched at least one file that is in scope.
/// 2. Its timestamp falls within `[ctx.since, ctx.until]`.
///
/// The OR-union is intentionally generous — agent-side curation handles
/// further trimming. Over-including is safer than losing context.
pub fn filter_by_scope(entries: &[JournalEntry], ctx: &GitContext) -> Vec<JournalEntry> {
    entries
        .iter()
        .filter(|e| in_time_window(e, ctx) || touches_scope_file(e, ctx))
        .cloned()
        .collect()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn in_time_window(entry: &JournalEntry, ctx: &GitContext) -> bool {
    entry.timestamp >= ctx.since && entry.timestamp <= ctx.until
}

fn touches_scope_file(entry: &JournalEntry, ctx: &GitContext) -> bool {
    entry
        .files_touched
        .iter()
        .any(|f| ctx.scope_files.contains(f))
}

/// Collect the union of files changed across all commits, deduplicated.
fn collect_files(commits: &[Commit]) -> Vec<String> {
    let mut files: Vec<String> = commits.iter().flat_map(|c| c.files.clone()).collect();
    files.sort();
    files.dedup();
    files
}

/// Return the timestamp of the earliest commit, or `None` if the list is empty.
fn earliest_commit_time(commits: &[Commit]) -> Option<DateTime<Utc>> {
    commits.iter().map(|c| c.timestamp).min()
}

/// Return the timestamp of the latest commit, or `None` if the list is empty.
fn latest_commit_time(commits: &[Commit]) -> Option<DateTime<Utc>> {
    commits.iter().map(|c| c.timestamp).max()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_entry(ts: DateTime<Utc>, files: &[&str], tool_calls: &[&str]) -> JournalEntry {
        JournalEntry {
            timestamp: ts,
            branch: "feature/test".to_string(),
            commit: "abc1234".to_string(),
            prompt: "test prompt".to_string(),
            files_touched: files.iter().map(|s| s.to_string()).collect(),
            tool_calls: tool_calls.iter().map(|s| s.to_string()).collect(),
            outcome: "done".to_string(),
            tool: "claude-code".to_string(),
            model: None,
            assistant_context: None,
        }
    }

    fn make_ctx(since: DateTime<Utc>, until: DateTime<Utc>, files: &[&str]) -> GitContext {
        GitContext {
            scope_files: files.iter().map(|s| s.to_string()).collect(),
            since,
            until,
            commits: Vec::new(),
        }
    }

    // Fixed reference point: 2024-01-15 12:00:00 UTC
    fn t(h: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap() + Duration::hours(h)
    }

    // ── filter_by_scope — time window ─────────────────────────────────────────

    #[test]
    fn test_filter_keeps_entry_in_time_window() {
        let ctx = make_ctx(t(0), t(2), &[]);
        let entries = vec![make_entry(t(1), &[], &[])]; // t(1) is between t(0) and t(2)
        let filtered = filter_by_scope(&entries, &ctx);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_drops_entry_outside_time_window_no_file_match() {
        let ctx = make_ctx(t(0), t(2), &[]);
        let entries = vec![make_entry(t(5), &[], &[])]; // t(5) is after t(2), no files
        let filtered = filter_by_scope(&entries, &ctx);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_keeps_entry_at_boundary() {
        let ctx = make_ctx(t(0), t(2), &[]);
        let at_start = make_entry(t(0), &[], &[]);
        let at_end = make_entry(t(2), &[], &[]);
        let filtered = filter_by_scope(&[at_start, at_end], &ctx);
        assert_eq!(filtered.len(), 2);
    }

    // ── filter_by_scope — file matching ───────────────────────────────────────

    #[test]
    fn test_filter_keeps_entry_matching_scoped_file() {
        // Entry is outside the time window but touches a scoped file.
        let ctx = make_ctx(t(0), t(1), &["src/auth.rs"]);
        let entries = vec![make_entry(t(5), &["src/auth.rs"], &[])];
        let filtered = filter_by_scope(&entries, &ctx);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_drops_entry_with_unrelated_file_outside_window() {
        let ctx = make_ctx(t(0), t(1), &["src/auth.rs"]);
        let entries = vec![make_entry(t(5), &["src/unrelated.rs"], &[])];
        let filtered = filter_by_scope(&entries, &ctx);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_keeps_entry_with_one_matching_file_among_many() {
        let ctx = make_ctx(t(0), t(1), &["src/auth.rs", "src/lib.rs"]);
        // Entry outside window, touches one scoped file and one unrelated.
        let entries = vec![make_entry(t(5), &["src/auth.rs", "src/other.rs"], &[])];
        let filtered = filter_by_scope(&entries, &ctx);
        assert_eq!(filtered.len(), 1);
    }

    // ── filter_by_scope — combined ────────────────────────────────────────────

    #[test]
    fn test_filter_keeps_entry_matching_either_condition() {
        let ctx = make_ctx(t(0), t(2), &["src/auth.rs"]);
        let in_window = make_entry(t(1), &[], &[]); // time match
        let file_match = make_entry(t(5), &["src/auth.rs"], &[]); // file match
        let neither = make_entry(t(5), &["src/other.rs"], &[]); // no match
        let filtered = filter_by_scope(&[in_window, file_match, neither], &ctx);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_empty_entries() {
        let ctx = make_ctx(t(0), t(2), &["src/auth.rs"]);
        let filtered = filter_by_scope(&[], &ctx);
        assert_eq!(filtered.len(), 0);
    }

    // ── build_git_context ─────────────────────────────────────────────────────

    #[test]
    fn test_build_git_context_last_n_commits_does_not_panic() {
        // Verifies the function runs without error in a normal repo context.
        let scope = ExtractionScope::LastNCommits(1);
        build_git_context(&scope).expect("build_git_context should succeed");
    }

    #[test]
    fn test_build_git_context_uncommitted_does_not_panic() {
        let scope = ExtractionScope::Uncommitted;
        build_git_context(&scope).expect("build_git_context should succeed");
    }

    #[test]
    fn test_build_git_context_since_is_before_until() {
        let scope = ExtractionScope::LastNCommits(2);
        let ctx = build_git_context(&scope).unwrap();
        assert!(ctx.since <= ctx.until);
    }

    // ── collect_files internal ────────────────────────────────────────────────

    #[test]
    fn test_collect_files_deduplicates() {
        use crate::analysis::git::Commit;
        let c1 = Commit {
            short_hash: "aaaaaaa".to_string(),
            message: "first".to_string(),
            timestamp: t(0),
            files: vec!["src/a.rs".to_string(), "src/b.rs".to_string()],
        };
        let c2 = Commit {
            short_hash: "bbbbbbb".to_string(),
            message: "second".to_string(),
            timestamp: t(1),
            files: vec!["src/b.rs".to_string(), "src/c.rs".to_string()],
        };
        let files = collect_files(&[c1, c2]);
        assert_eq!(files, vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
    }
}
