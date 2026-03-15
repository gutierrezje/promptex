//! JSON serialization for `pmtx extract`.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::analysis::correlation::GitContext;
use crate::analysis::scope::ExtractionScope;
use crate::prompt::PromptEntry;

/// Top-level JSON envelope emitted by `pmtx extract`.
#[derive(Serialize)]
struct JsonOutput<'a> {
    /// Resolved scope kind.
    scope: &'static str,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    commits: Vec<CommitSummary>,
    scope_files: &'a [String],
    entries: &'a [PromptEntry],
}

#[derive(Serialize)]
struct CommitSummary {
    short_hash: String,
    message: String,
}

/// Serialize correlated entries to JSON for agent-side processing.
pub fn render_json(
    entries: &[PromptEntry],
    ctx: &GitContext,
    scope: &ExtractionScope,
) -> anyhow::Result<String> {
    let commits = ctx
        .commits
        .iter()
        .map(|c| CommitSummary {
            short_hash: c.short_hash.clone(),
            message: c.message.clone(),
        })
        .collect();

    let output = JsonOutput {
        scope: scope_label(scope),
        since: ctx.since,
        until: ctx.until,
        commits,
        scope_files: &ctx.scope_files,
        entries,
    };

    Ok(serde_json::to_string_pretty(&output)?)
}

fn scope_label(scope: &ExtractionScope) -> &'static str {
    match scope {
        ExtractionScope::BranchLifetime { .. } => "branch-lifetime",
        ExtractionScope::LastNCommits(_) => "last-n-commits",
        ExtractionScope::SinceCommit(_) => "since-commit",
        ExtractionScope::Uncommitted => "uncommitted",
        ExtractionScope::SinceTime(_) => "since-time",
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::render_json;
    use crate::analysis::correlation::GitContext;
    use crate::analysis::git::Commit;
    use crate::analysis::scope::ExtractionScope;
    use crate::prompt::PromptEntry;

    #[test]
    fn render_json_includes_empty_entries_array() {
        let since = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 1, 11, 0, 0).unwrap();
        let ctx = GitContext {
            scope_files: vec!["src/lib.rs".to_string()],
            since,
            until,
            commits: vec![Commit {
                short_hash: "abc1234".to_string(),
                message: "feat: test commit".to_string(),
                timestamp: since,
                files: vec!["src/lib.rs".to_string()],
            }],
        };

        let entries: Vec<PromptEntry> = Vec::new();
        let json = render_json(&entries, &ctx, &ExtractionScope::LastNCommits(1)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["scope"], "last-n-commits");
        assert!(value["entries"].is_array());
        assert_eq!(value["entries"].as_array().unwrap().len(), 0);
        assert_eq!(value["scope_files"][0], "src/lib.rs");
        assert_eq!(value["commits"][0]["short_hash"], "abc1234");
    }
}
