use std::env;

use anyhow::Result;

use crate::analysis::correlation::{build_git_context, filter_by_scope};
use crate::analysis::scope::{determine_scope, ExtractionScope, ScopeFlags};
use crate::extractors;
use crate::project_id;

pub fn execute(
    uncommitted: bool,
    commits: Option<usize>,
    since_commit: Option<String>,
    branch_lifetime: bool,
    _write: Option<Option<String>>,
) -> Result<()> {
    let cwd = env::current_dir()?;

    // ── Step 1: Determine scope ───────────────────────────────────────────────
    let flags = ScopeFlags { uncommitted, commits, since_commit, branch_lifetime };
    let scope = determine_scope(&flags)?;

    eprintln!("🔍 Analyzing workspace...");
    match &scope {
        ExtractionScope::BranchLifetime { branch, since_commit } => {
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
    }

    // ── Step 2: Resolve scope into files + time window ────────────────────────
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

    // ── Step 3: Detect extractor and pull raw entries ─────────────────────────
    let pid = project_id::get_project_id(&cwd)?;
    let extractor = extractors::detect(&cwd, &pid);
    eprintln!("\n🔎 Loading journal ({})...", extractor.kind.label());

    let raw_entries = extractor.extract(ctx.since, ctx.until)?;
    eprintln!("  ✓ Found {} entries in time range", raw_entries.len());

    // ── Step 4: Correlate — filter to scope (Phase 6) ─────────────────────────
    let entries = filter_by_scope(&raw_entries, &ctx);
    eprintln!("  ✓ Filtered to {} relevant entries", entries.len());

    // ── TODO Phase 7: curate (categorize, deduplicate) ────────────────────────
    // ── TODO Phase 8: format and output ──────────────────────────────────────

    if entries.is_empty() {
        eprintln!("\nNo prompts found for this scope.");
        eprintln!("Try widening the scope with --commits N or --branch-lifetime.");
        return Ok(());
    }

    // Temporary plain-text preview until output formatting is implemented.
    eprintln!("\n📝 Prompts in scope ({} entries):", entries.len());
    for e in &entries {
        println!(
            "[{}] ({}) {}",
            e.timestamp.format("%H:%M"),
            e.tool,
            e.prompt,
        );
    }

    Ok(())
}
