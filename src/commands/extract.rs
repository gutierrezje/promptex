use std::env;

use anyhow::Result;

use crate::analysis::correlation::{build_git_context, filter_by_scope};
use crate::analysis::scope::{determine_scope, ExtractionScope, ScopeFlags};
use crate::extractors::{self, ExtractionDiagnostics};
use crate::output::json_format;
use crate::project_id;

/// Run the `pmtx extract` command with resolved scope flags.
pub fn execute(
    uncommitted: bool,
    commits: Option<usize>,
    since_commit: Option<String>,
    branch_lifetime: bool,
    since_duration: Option<String>,
) -> Result<()> {
    let cwd = env::current_dir()?;

    let flags = ScopeFlags {
        uncommitted,
        commits,
        since_commit,
        branch_lifetime,
        since_duration,
    };
    let scope = determine_scope(&flags)?;

    eprintln!("Analyzing workspace...");
    match &scope {
        ExtractionScope::BranchLifetime {
            branch,
            since_commit,
        } => {
            eprintln!("  * Branch: {branch} (since {})", &since_commit[..7]);
        }
        ExtractionScope::LastNCommits(n) => {
            eprintln!("  * Scope: last {n} commit(s)");
        }
        ExtractionScope::SinceCommit(hash) => {
            eprintln!("  * Scope: since commit {hash}");
        }
        ExtractionScope::Uncommitted => {
            eprintln!("  * Scope: uncommitted changes only");
        }
        ExtractionScope::SinceTime(since) => {
            eprintln!("  * Scope: since {}", since.format("%Y-%m-%d %H:%M UTC"));
        }
    }

    let ctx = build_git_context(&scope)?;
    eprintln!(
        "  * Time range: {} → {}",
        ctx.since.format("%Y-%m-%d %H:%M"),
        ctx.until.format("%Y-%m-%d %H:%M"),
    );
    if !ctx.commits.is_empty() {
        eprintln!("  * {} commit(s) in scope", ctx.commits.len());
    }
    if !ctx.scope_files.is_empty() {
        eprintln!("  * {} file(s) in scope", ctx.scope_files.len());
    }

    let pid = project_id::get_project_id(&cwd)?;
    let extractor = extractors::detect(&cwd, &pid);
    let kind_label = extractor
        .primary_kind()
        .map(|k| k.label())
        .unwrap_or("none");
    eprintln!("\nLoading prompts ({kind_label})...");

    let (contributing, mut raw_entries, diagnostics) =
        extractor.extract_all(ctx.since, ctx.until)?;
    for (kind, count) in &contributing {
        eprintln!("  * {} — {count} entries", kind.label());
    }
    if contributing.is_empty() {
        eprintln!("  * No entries found");
    }
    eprintln!("  * {} total in time range", raw_entries.len());

    print_warning_summary(&diagnostics);

    let mut seen_ids = std::collections::HashSet::new();
    for entry in &mut raw_entries {
        let base_id = format!("{}-{}", entry.tool, entry.timestamp.timestamp_millis());
        let mut final_id = base_id.clone();
        let mut counter = 1;
        while seen_ids.contains(&final_id) {
            final_id = format!("{}-{}", base_id, counter);
            counter += 1;
        }
        seen_ids.insert(final_id.clone());
        entry.id = final_id;
    }

    let entries = filter_by_scope(&raw_entries, &ctx);
    eprintln!("  * Filtered to {} relevant entries", entries.len());

    if entries.is_empty() {
        eprintln!("\nNo prompts found for this scope.");
        eprintln!("Try widening the scope with --commits N or --branch-lifetime.");
    }

    let out = json_format::render_json(&entries, &ctx, &scope, &diagnostics)?;
    println!("{out}");

    Ok(())
}

/// Limit for warning previews shown on stderr to avoid overwhelming the user.
const MAX_WARNING_PREVIEW: usize = 3;

/// Finalize extractor diagnostics policy:
/// 1. Extractor parse issues (e.g. malformed JSON lines) are non-fatal.
/// 2. Full details of every warning are always included in the JSON output.
/// 3. stderr prints a bounded human-readable summary of these warnings.
fn print_warning_summary(diagnostics: &ExtractionDiagnostics) {
    for line in warning_summary_lines(diagnostics) {
        eprintln!("{line}");
    }
}

fn warning_summary_lines(diagnostics: &ExtractionDiagnostics) -> Vec<String> {
    if diagnostics.warnings.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![
        format!(
            "\n  [!] {} non-fatal parse warning(s) occurred during extraction.",
            diagnostics.warnings.len()
        ),
        "      See JSON output warnings for complete details.".to_string(),
    ];

    for (source, count) in diagnostics.warning_count_by_source() {
        lines.push(format!("    - {}: {count}", source.label()));
    }

    for warning in diagnostics.warnings.iter().take(MAX_WARNING_PREVIEW) {
        lines.push(format!(
            "    · {}: {}",
            warning.source.label(),
            warning.detail
        ));
    }

    let remaining = diagnostics
        .warnings
        .len()
        .saturating_sub(MAX_WARNING_PREVIEW);
    if remaining > 0 {
        lines.push(format!("    · ... and {remaining} more"));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::{ExtractionDiagnostics, ExtractionWarning, ExtractorKind};
    use chrono::Utc;
    use std::path::Path;

    #[test]
    fn test_print_warning_summary_empty() {
        let diagnostics = ExtractionDiagnostics::default();
        let lines = warning_summary_lines(&diagnostics);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_print_warning_summary_with_overflow() {
        let mut diagnostics = ExtractionDiagnostics::default();
        for i in 0..5 {
            diagnostics.warnings.push(ExtractionWarning {
                source: ExtractorKind::ClaudeCode,
                detail: format!("warning {}", i),
            });
        }

        let lines = warning_summary_lines(&diagnostics);

        assert!(lines
            .iter()
            .any(|line| line.contains("5 non-fatal parse warning(s)")));
        assert!(lines
            .iter()
            .any(|line| line.contains("See JSON output warnings for complete details.")));
        assert!(lines.iter().any(|line| line == "    - Claude Code: 5"));
        assert!(lines.iter().any(|line| line.contains("warning 0")));
        assert!(lines.iter().any(|line| line.contains("warning 1")));
        assert!(lines.iter().any(|line| line.contains("warning 2")));
        assert!(!lines.iter().any(|line| line.contains("warning 3")));
        assert!(!lines.iter().any(|line| line.contains("warning 4")));
        assert!(lines.iter().any(|line| line.contains("... and 2 more")));
    }

    #[test]
    fn test_print_warning_summary_groups_by_source() {
        let mut diagnostics = ExtractionDiagnostics::default();
        diagnostics.warnings.push(ExtractionWarning {
            source: ExtractorKind::ClaudeCode,
            detail: "a".to_string(),
        });
        diagnostics.warnings.push(ExtractionWarning {
            source: ExtractorKind::Codex,
            detail: "b".to_string(),
        });
        diagnostics.warnings.push(ExtractionWarning {
            source: ExtractorKind::Codex,
            detail: "c".to_string(),
        });

        let lines = warning_summary_lines(&diagnostics);

        assert!(lines.iter().any(|line| line == "    - Claude Code: 1"));
        assert!(lines
            .iter()
            .any(|line| line == "    - Codex CLI / Desktop: 2"));
        assert!(!lines.iter().any(|line| line.contains("... and")));
    }

    #[test]
    fn test_contract_helper_for_normalized_entries() {
        use chrono::TimeZone;
        let since = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2024, 1, 15, 14, 0, 0).unwrap();
        let project_root = Path::new("/proj");

        let entry = crate::prompt::PromptEntry::new(
            "main".to_string(),
            "abc".to_string(),
            "fix it".to_string(),
            vec!["src/lib.rs".to_string()],
            vec!["Write".to_string()],
            "claude-code".to_string(),
            None,
        );
        let mut entry = entry;
        entry.timestamp = since + chrono::Duration::hours(1);

        let entries = vec![entry];
        crate::extractors::test_contract::assert_entries_contract(
            &entries,
            project_root,
            since,
            until,
        );
    }
}
