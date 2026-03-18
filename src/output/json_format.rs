//! JSON serialization for `pmtx extract`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::analysis::correlation::GitContext;
use crate::analysis::scope::ExtractionScope;
use crate::extractors::{ExtractionDiagnostics, ExtractionWarning};
use crate::prompt::PromptEntry;

/// Top-level JSON envelope emitted by `pmtx extract`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractionReport {
    /// Resolved scope kind.
    pub scope: String,
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
    pub commits: Vec<CommitSummary>,
    pub scope_files: Vec<String>,
    pub entries: Vec<PromptEntry>,
    pub warnings: Vec<ExtractionWarning>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitSummary {
    pub short_hash: String,
    pub message: String,
}

/// Serialize correlated entries to JSON for agent-side processing.
pub fn render_json(
    entries: &[PromptEntry],
    ctx: &GitContext,
    scope: &ExtractionScope,
    diagnostics: &ExtractionDiagnostics,
) -> anyhow::Result<String> {
    let commits = ctx
        .commits
        .iter()
        .map(|c| CommitSummary {
            short_hash: c.short_hash.clone(),
            message: c.message.clone(),
        })
        .collect();

    let output = ExtractionReport {
        scope: scope_label(scope),
        since: ctx.since,
        until: ctx.until,
        commits,
        scope_files: ctx.scope_files.clone(),
        entries: entries.to_vec(),
        warnings: diagnostics.warnings.clone(),
    };

    Ok(serde_json::to_string_pretty(&output)?)
}

fn scope_label(scope: &ExtractionScope) -> String {
    match scope {
        ExtractionScope::BranchLifetime { .. } => "branch-lifetime".to_string(),
        ExtractionScope::LastNCommits(_) => "last-n-commits".to_string(),
        ExtractionScope::SinceCommit(_) => "since-commit".to_string(),
        ExtractionScope::Uncommitted => "uncommitted".to_string(),
        ExtractionScope::SinceTime(_) => "since-time".to_string(),
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
        let json = render_json(
            &entries,
            &ctx,
            &ExtractionScope::LastNCommits(1),
            &crate::extractors::ExtractionDiagnostics::default(),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["scope"], "last-n-commits");
        assert!(value["entries"].is_array());
        assert_eq!(value["entries"].as_array().unwrap().len(), 0);
        assert_eq!(value["scope_files"][0], "src/lib.rs");
        assert_eq!(value["commits"][0]["short_hash"], "abc1234");
    }

    #[test]
    fn render_json_includes_warnings() {
        let since = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 1, 11, 0, 0).unwrap();
        let ctx = GitContext {
            scope_files: vec![],
            since,
            until,
            commits: vec![],
        };

        let mut diagnostics = crate::extractors::ExtractionDiagnostics::default();
        diagnostics
            .warnings
            .push(crate::extractors::ExtractionWarning {
                source: crate::extractors::ExtractorKind::ClaudeCode,
                detail: "bad line".to_string(),
            });

        let entries: Vec<PromptEntry> = Vec::new();
        let json = render_json(
            &entries,
            &ctx,
            &ExtractionScope::LastNCommits(1),
            &diagnostics,
        )
        .unwrap();

        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value["warnings"].is_array());
        let warnings = value["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0]["source"], "claude-code");
        assert_eq!(warnings[0]["detail"], "bad line");
    }

    #[test]
    fn extraction_report_roundtrips() {
        let since = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 1, 11, 0, 0).unwrap();
        let ctx = GitContext {
            scope_files: vec!["src/main.rs".to_string()],
            since,
            until,
            commits: vec![Commit {
                short_hash: "abc1234".to_string(),
                message: "feat: init".to_string(),
                timestamp: since,
                files: vec!["src/main.rs".to_string()],
            }],
        };

        let mut entry = PromptEntry::new(
            "main".to_string(),
            "".to_string(),
            "do something".to_string(),
            vec![],
            vec![],
            "codex".to_string(),
            None,
        );
        entry.category = Some("Investigation".to_string());
        let entries = vec![entry];
        let diagnostics = crate::extractors::ExtractionDiagnostics::default();

        let json = render_json(
            &entries,
            &ctx,
            &ExtractionScope::LastNCommits(1),
            &diagnostics,
        )
        .unwrap();

        let deserialized: super::ExtractionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.scope, "last-n-commits");
        assert_eq!(deserialized.entries.len(), 1);
        assert_eq!(
            deserialized.entries[0].category.as_deref(),
            Some("Investigation")
        );
    }
}
