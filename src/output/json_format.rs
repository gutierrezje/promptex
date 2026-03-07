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
