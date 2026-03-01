//! JSON output for `pmtx extract --json`.
//!
//! Emits curated, pre-dedup entries as structured JSON so an agent can
//! perform semantic categorization (Investigation / Solution / Testing)
//! and render the final PR markdown following the pr_format spec.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::analysis::correlation::GitContext;
use crate::analysis::scope::ExtractionScope;
use crate::journal::JournalEntry;

/// Top-level JSON envelope emitted by `pmtx extract --json`.
#[derive(Serialize)]
struct JsonOutput<'a> {
    /// String label for the resolved scope kind.
    scope: &'static str,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    commits: Vec<CommitSummary>,
    scope_files: &'a [String],
    /// Curated entries (artifact-filtered + deduped, not yet categorized).
    entries: &'a [JournalEntry],
    /// Rendering spec the agent should follow when producing PR markdown.
    format_spec: FormatSpec,
}

#[derive(Serialize)]
struct CommitSummary {
    short_hash: String,
    message: String,
}

#[derive(Serialize)]
struct FormatSpec {
    categories: [&'static str; 3],
    entry_format: &'static str,
    header: &'static str,
    footer: &'static str,
}

static FORMAT_SPEC: FormatSpec = FormatSpec {
    categories: ["Investigation", "Solution", "Testing"],
    entry_format: "**[HH:MM] (Tool · Model)**\\n> prompt line 1\\n> prompt line 2\\n\\n→ outcome (if non-empty)\\n→ Files: `file` (if non-empty)\\n→ Commit: `short_hash` (if non-empty)",
    header: "## 🤖 Prompt History\\n\\n<details>\\n<summary>N prompts over Xh Ym - Click to expand</summary>\\n\\n**Session Details**\\n- Tools: ...- Branch: ...- Time range: ...- Commits: ...- Modified files: ...",
    footer: "---\\n\\n*Generated with [PromptEx](https://github.com/gutierrezje/promptex)*",
};

/// Serialize curated entries to JSON for agent-side categorization.
pub fn render_json(
    entries: &[JournalEntry],
    ctx: &GitContext,
    scope: &ExtractionScope,
) -> anyhow::Result<String> {
    let commits = ctx.commits.iter()
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
        format_spec: FormatSpec {
            categories: FORMAT_SPEC.categories,
            entry_format: FORMAT_SPEC.entry_format,
            header: FORMAT_SPEC.header,
            footer: FORMAT_SPEC.footer,
        },
    };

    Ok(serde_json::to_string_pretty(&output)?)
}

fn scope_label(scope: &ExtractionScope) -> &'static str {
    match scope {
        ExtractionScope::BranchLifetime { .. } => "branch-lifetime",
        ExtractionScope::LastNCommits(_)       => "last-n-commits",
        ExtractionScope::SinceCommit(_)        => "since-commit",
        ExtractionScope::Uncommitted           => "uncommitted",
        ExtractionScope::SinceTime(_)          => "since-time",
    }
}
