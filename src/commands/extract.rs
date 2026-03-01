use std::env;

use anyhow::Result;

use crate::analysis::correlation::{build_git_context, filter_by_scope};
use crate::analysis::scope::{determine_scope, ExtractionScope, ScopeFlags};
use crate::curation::categorize::{categorize, Intent};
use crate::curation::filter::{apply_artifact_filter, remove_duplicates};
use crate::extractors;
use crate::output::interactive;
use crate::output::pr_format;
use crate::project_id;

pub fn execute(
    uncommitted: bool,
    commits: Option<usize>,
    since_commit: Option<String>,
    branch_lifetime: bool,
    since_duration: Option<String>,
    write_to: Option<Option<String>>,
) -> Result<()> {
    let cwd = env::current_dir()?;

    // ── Step 1: Determine scope ───────────────────────────────────────────────
    let flags = ScopeFlags { uncommitted, commits, since_commit, branch_lifetime, since_duration };
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
        ExtractionScope::SinceTime(since) => {
            eprintln!("  ✓ Scope: since {}", since.format("%Y-%m-%d %H:%M UTC"));
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

    let (source_kind, raw_entries) = extractor.extract_with_source(ctx.since, ctx.until)?;
    if source_kind != extractor.kind {
        eprintln!("  ↪ Primary source had no entries; switched to {}.", source_kind.label());
    }
    eprintln!("  ✓ Found {} entries in time range", raw_entries.len());

    // ── Step 4: Correlate — filter to scope (Phase 6) ─────────────────────────
    let entries = filter_by_scope(&raw_entries, &ctx);
    eprintln!("  ✓ Filtered to {} relevant entries", entries.len());

    // ── Step 5: Curate — artifact filter + deduplication (Phase 7) ───────────
    let entries = apply_artifact_filter(entries);
    let entries = remove_duplicates(entries);

    if entries.is_empty() {
        eprintln!("\nNo prompts found for this scope.");
        eprintln!("Try widening the scope with --commits N or --branch-lifetime.");
        return Ok(());
    }

    // ── Step 6: Categorize (Phase 7) ──────────────────────────────────────────
    let mut investigations: Vec<_> = Vec::new();
    let mut solutions: Vec<_> = Vec::new();
    let mut tests: Vec<_> = Vec::new();

    for e in &entries {
        match categorize(e) {
            Intent::Investigation => investigations.push(e),
            Intent::Solution => solutions.push(e),
            Intent::Testing => tests.push(e),
        }
    }

    eprintln!("\n📝 Curating prompt log...");
    eprintln!("  Investigation: {} prompts", investigations.len());
    eprintln!("  Solution: {} prompts", solutions.len());
    eprintln!("  Testing: {} prompts", tests.len());

    // ── Step 7: Render and output (Phase 8) ──────────────────────────────────
    let markdown = pr_format::render(&investigations, &solutions, &tests, &ctx, &scope);

    match write_to {
        Some(Some(path)) => {
            std::fs::write(&path, &markdown)?;
            eprintln!("\n✓ Written to {path}");
        }
        Some(None) => {
            std::fs::write("PROMPTS.md", &markdown)?;
            eprintln!("\n✓ Written to PROMPTS.md");
        }
        None => {
            println!("{markdown}");
            interactive::maybe_prompt(&markdown)?;
        }
    }

    Ok(())
}
