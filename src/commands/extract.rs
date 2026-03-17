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

    eprintln!("🔍 Analyzing workspace...");
    match &scope {
        ExtractionScope::BranchLifetime {
            branch,
            since_commit,
        } => {
            eprintln!("  ✓ Branch: {branch} (since {})", &since_commit[..7]);
        }
        ExtractionScope::LastNCommits(n) => {
            eprintln!("  ✓ Scope: last {n} commit(s)");
        }
        ExtractionScope::SinceCommit(hash) => {
            eprintln!("  ✓ Scope: since commit {hash}");
        }
        ExtractionScope::Uncommitted => {
            eprintln!("  ✓ Scope: uncommitted changes only");
        }
        ExtractionScope::SinceTime(since) => {
            eprintln!("  ✓ Scope: since {}", since.format("%Y-%m-%d %H:%M UTC"));
        }
    }

    let ctx = build_git_context(&scope)?;
    eprintln!(
        "  ✓ Time range: {} → {}",
        ctx.since.format("%Y-%m-%d %H:%M"),
        ctx.until.format("%Y-%m-%d %H:%M"),
    );
    if !ctx.commits.is_empty() {
        eprintln!("  ✓ {} commit(s) in scope", ctx.commits.len());
    }
    if !ctx.scope_files.is_empty() {
        eprintln!("  ✓ {} file(s) in scope", ctx.scope_files.len());
    }

    let pid = project_id::get_project_id(&cwd)?;
    let extractor = extractors::detect(&cwd, &pid);
    let kind_label = extractor
        .primary_kind()
        .map(|k| k.label())
        .unwrap_or("none");
    eprintln!("\n🔎 Loading prompts ({kind_label})...");

    let (contributing, raw_entries, diagnostics) = extractor.extract_all(ctx.since, ctx.until)?;
    for (kind, count) in &contributing {
        eprintln!("  ✓ {} — {count} entries", kind.label());
    }
    if contributing.is_empty() {
        eprintln!("  ✓ No entries found");
    }
    eprintln!("  ✓ {} total in time range", raw_entries.len());

    print_warning_summary(&diagnostics);

    let entries = filter_by_scope(&raw_entries, &ctx);
    eprintln!("  ✓ Filtered to {} relevant entries", entries.len());

    if entries.is_empty() {
        eprintln!("\nNo prompts found for this scope.");
        eprintln!("Try widening the scope with --commits N or --branch-lifetime.");
    }

    let out = json_format::render_json(&entries, &ctx, &scope)?;
    println!("{out}");

    Ok(())
}

fn print_warning_summary(diagnostics: &ExtractionDiagnostics) {
    if diagnostics.warnings.is_empty() {
        return;
    }

    eprintln!(
        "  ⚠ {} non-fatal extraction warning(s)",
        diagnostics.warnings.len()
    );

    for (source, count) in diagnostics.warning_count_by_source() {
        eprintln!("    - {}: {count}", source.label());
    }

    for warning in diagnostics.warnings.iter().take(3) {
        eprintln!("    · {}: {}", warning.source.label(), warning.detail);
    }

    let remaining = diagnostics.warnings.len().saturating_sub(3);
    if remaining > 0 {
        eprintln!("    · ... and {remaining} more");
    }
}
