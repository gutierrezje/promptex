//! PR-ready markdown output for `pmtx extract`.
//!
//! `render` is the single entry point: it takes the categorized prompt groups
//! and the git context, and returns a fully-formed markdown string. The caller
//! decides whether to print it to stdout or write it to a file.

use std::collections::HashMap;
use std::collections::HashSet;

use chrono::Duration;

use crate::analysis::correlation::GitContext;
use crate::analysis::scope::ExtractionScope;
use crate::journal::JournalEntry;

// ── Public API ─────────────────────────────────────────────────────────────────

/// Render categorized prompt groups into PR-ready collapsible markdown.
///
/// Output format:
/// ```markdown
/// ## 🤖 Prompt History
///
/// <details>
/// <summary>N prompts over Xh Ym - Click to expand</summary>
///
/// **Session Details**
/// ...
///
/// ### 🔍 Investigation
/// **[HH:MM] (Tool) Title**
/// ```
/// prompt text
/// ```
/// → outcome
/// → Files: `file`
/// ...
///
/// </details>
/// ```
pub fn render(
    investigations: &[&JournalEntry],
    solutions: &[&JournalEntry],
    tests: &[&JournalEntry],
    ctx: &GitContext,
    scope: &ExtractionScope,
) -> String {
    let all: Vec<&JournalEntry> = investigations
        .iter()
        .chain(solutions.iter())
        .chain(tests.iter())
        .copied()
        .collect();

    let total = all.len();
    let duration_str = format_duration(ctx.until.signed_duration_since(ctx.since));
    let plural = if total == 1 { "" } else { "s" };

    let mut md = String::new();

    // ── Header ──────────────────────────────────────────────────────────────
    md.push_str("## 🤖 Prompt History\n\n");
    md.push_str(&format!(
        "<details>\n<summary>{total} prompt{plural} over {duration_str} - Click to expand</summary>\n\n"
    ));

    // ── Session details ──────────────────────────────────────────────────────
    md.push_str("**Session Details**\n");
    md.push_str(&format!("- Tools: {}\n", tool_summary(&all)));

    if let Some(branch) = branch_from(scope, &all) {
        md.push_str(&format!("- Branch: `{branch}`\n"));
    }

    md.push_str(&format!(
        "- Time range: {} - {}\n",
        ctx.since.format("%Y-%m-%d %H:%M"),
        ctx.until.format("%Y-%m-%d %H:%M"),
    ));

    if !ctx.commits.is_empty() {
        let refs: Vec<String> = ctx.commits.iter()
            .map(|c| format!("`{}`", c.short_hash))
            .collect();
        let n = ctx.commits.len();
        md.push_str(&format!(
            "- Commits: {} ({n} commit{})\n",
            refs.join(", "),
            if n == 1 { "" } else { "s" },
        ));
    }

    if !ctx.scope_files.is_empty() {
        let shown: Vec<String> = ctx.scope_files.iter()
            .take(8)
            .map(|f| format!("`{f}`"))
            .collect();
        let overflow = if ctx.scope_files.len() > 8 {
            format!(" +{} more", ctx.scope_files.len() - 8)
        } else {
            String::new()
        };
        md.push_str(&format!("- Modified files: {}{overflow}\n", shown.join(", ")));
    }

    // ── Categorized sections ─────────────────────────────────────────────────
    for (label, emoji, group) in [
        ("Investigation", "🔍", investigations),
        ("Solution", "🔧", solutions),
        ("Testing", "✅", tests),
    ] {
        if group.is_empty() {
            continue;
        }
        md.push_str(&format!("\n---\n\n### {emoji} {label}\n\n"));
        for e in group.iter() {
            md.push_str(&format_entry(e));
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────────
    let tool_count = unique_tool_count(&all);
    md.push_str(&format!(
        "\n---\n\n**Summary:** {total} prompt{plural} ({} investigation, {} solution, {} testing) · {tool_count} tool{}\n\n",
        investigations.len(),
        solutions.len(),
        tests.len(),
        if tool_count == 1 { "" } else { "s" },
    ));

    md.push_str("</details>\n");
    md
}

// ── Entry formatting ──────────────────────────────────────────────────────────

fn format_entry(e: &JournalEntry) -> String {
    let mut s = String::new();

    let tool_name = tool_display_name(&e.tool);
    let model_suffix = e.model.as_deref()
        .map(|m| format!(" · {m}"))
        .unwrap_or_default();
    let title = derive_title(&e.prompt);

    s.push_str(&format!(
        "**[{}] ({}{}) {}**\n",
        e.timestamp.format("%H:%M"),
        tool_name,
        model_suffix,
        title,
    ));

    // Prompt block
    s.push_str("```\n");
    s.push_str(e.prompt.trim());
    s.push_str("\n```\n");

    // Outcome
    if !e.outcome.is_empty() {
        s.push_str(&format!("→ {}\n", e.outcome));
    }

    // Files touched
    if !e.files_touched.is_empty() {
        let files: Vec<String> = e.files_touched.iter()
            .map(|f| format!("`{f}`"))
            .collect();
        s.push_str(&format!("→ Files: {}\n", files.join(", ")));
    }

    // Commit (only when it looks like a real hash)
    if looks_like_commit(&e.commit) {
        let short = if e.commit.len() >= 7 { &e.commit[..7] } else { &e.commit };
        s.push_str(&format!("→ Commit: `{short}`\n"));
    }

    s.push('\n');
    s
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Shorten and capitalise a prompt into a one-line title.
///
/// Strips common polite fillers ("can you", "please", etc.) from the start,
/// then truncates to 60 characters.
fn derive_title(prompt: &str) -> String {
    let first_line = prompt.lines().next().unwrap_or(prompt).trim();
    let stripped = strip_filler(first_line);
    let capitalised = capitalise(stripped);

    if capitalised.chars().count() > 60 {
        let truncated: String = capitalised.chars().take(57).collect();
        format!("{truncated}...")
    } else {
        capitalised
    }
}

fn strip_filler(s: &str) -> &str {
    let lower = s.to_lowercase();
    for filler in &[
        "can you ", "could you ", "please ", "i need you to ", "i want you to ",
        "would you ", "help me ", "i'd like you to ",
    ] {
        if lower.starts_with(filler) {
            return &s[filler.len()..];
        }
    }
    s
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Human-readable name for a tool slug.
fn tool_display_name(tool: &str) -> String {
    match tool {
        "claude-code" => "Claude Code".to_string(),
        "opencode"    => "OpenCode".to_string(),
        "codex"       => "Codex".to_string(),
        "cursor"      => "Cursor".to_string(),
        "copilot"     => "GitHub Copilot".to_string(),
        other         => other.to_string(),
    }
}

/// Aggregate entries by (tool, model) and return a compact summary string.
///
/// Example: `"Claude Code (claude-sonnet-4.5) - 5 prompts, Cursor - 1 prompt"`
fn tool_summary(entries: &[&JournalEntry]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in entries {
        let key = match e.model.as_deref() {
            Some(m) => format!("{} ({})", tool_display_name(&e.tool), m),
            None    => tool_display_name(&e.tool),
        };
        *counts.entry(key).or_insert(0) += 1;
    }

    // Sort by count desc, then name asc — deterministic output for tests
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    pairs.iter()
        .map(|(name, n)| format!("{name} - {n} prompt{}", if *n == 1 { "" } else { "s" }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Count distinct tool slugs across all entries.
fn unique_tool_count(entries: &[&JournalEntry]) -> usize {
    entries.iter().map(|e| e.tool.as_str()).collect::<HashSet<_>>().len()
}

/// Best-effort branch name: prefer scope metadata, fall back to first entry.
fn branch_from(scope: &ExtractionScope, entries: &[&JournalEntry]) -> Option<String> {
    if let ExtractionScope::BranchLifetime { branch, .. } = scope {
        return Some(branch.clone());
    }
    entries.first()
        .map(|e| e.branch.clone())
        .filter(|b| !b.is_empty())
}

/// Format a `chrono::Duration` as a human-readable string ("1h 46m", "32m").
fn format_duration(d: Duration) -> String {
    let mins = d.num_minutes().max(0);
    if mins < 1 {
        "< 1m".to_string()
    } else if mins < 60 {
        format!("{mins}m")
    } else {
        let h = mins / 60;
        let m = mins % 60;
        if m == 0 { format!("{h}h") } else { format!("{h}h {m}m") }
    }
}

/// Return true if `s` looks like a git commit hash (hex, ≥ 7 chars).
fn looks_like_commit(s: &str) -> bool {
    s.len() >= 7 && s.chars().all(|c| c.is_ascii_hexdigit())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    use crate::analysis::git::Commit;

    fn t(h: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 15, 14, 0, 0).unwrap()
            + chrono::Duration::hours(h)
    }

    fn entry(prompt: &str, tool: &str, model: Option<&str>, tool_calls: &[&str], files: &[&str], outcome: &str) -> JournalEntry {
        JournalEntry {
            timestamp: t(0),
            branch: "feature/auth-fix".to_string(),
            commit: "abc1234def567".to_string(),
            prompt: prompt.to_string(),
            files_touched: files.iter().map(|s| s.to_string()).collect(),
            tool_calls: tool_calls.iter().map(|s| s.to_string()).collect(),
            outcome: outcome.to_string(),
            tool: tool.to_string(),
            model: model.map(|s| s.to_string()),
        }
    }

    fn make_ctx(commits: Vec<Commit>, files: Vec<&str>) -> GitContext {
        GitContext {
            scope_files: files.iter().map(|s| s.to_string()).collect(),
            since: t(0),
            until: t(2),
            commits,
        }
    }

    fn make_commit(short: &str) -> Commit {
        Commit {
            hash: format!("{short}0000000000000000000000000000000000"),
            short_hash: short.to_string(),
            message: "feat: something".to_string(),
            timestamp: t(0),
            files: vec![],
        }
    }

    // ── format_duration ───────────────────────────────────────────────────

    #[test]
    fn test_duration_under_a_minute() {
        assert_eq!(format_duration(Duration::seconds(30)), "< 1m");
    }

    #[test]
    fn test_duration_minutes_only() {
        assert_eq!(format_duration(Duration::minutes(32)), "32m");
    }

    #[test]
    fn test_duration_hours_and_minutes() {
        assert_eq!(format_duration(Duration::minutes(106)), "1h 46m");
    }

    #[test]
    fn test_duration_exact_hours() {
        assert_eq!(format_duration(Duration::hours(2)), "2h");
    }

    // ── derive_title ──────────────────────────────────────────────────────

    #[test]
    fn test_title_capitalises_first_letter() {
        assert_eq!(derive_title("implement jwt validation"), "Implement jwt validation");
    }

    #[test]
    fn test_title_strips_please() {
        assert_eq!(derive_title("please fix the auth bug"), "Fix the auth bug");
    }

    #[test]
    fn test_title_strips_can_you() {
        assert_eq!(derive_title("can you add a test"), "Add a test");
    }

    #[test]
    fn test_title_truncates_long_prompt() {
        let long = "implement the full jwt validation with expiry checking and token refresh logic here";
        let title = derive_title(long);
        assert!(title.ends_with("..."));
        assert!(title.chars().count() <= 60);
    }

    #[test]
    fn test_title_uses_first_line_only() {
        let multiline = "fix auth bug\nmore details on line two";
        assert_eq!(derive_title(multiline), "Fix auth bug");
    }

    // ── tool_display_name ─────────────────────────────────────────────────

    #[test]
    fn test_known_tool_names() {
        assert_eq!(tool_display_name("claude-code"), "Claude Code");
        assert_eq!(tool_display_name("codex"), "Codex");
        assert_eq!(tool_display_name("cursor"), "Cursor");
    }

    #[test]
    fn test_unknown_tool_name_passthrough() {
        assert_eq!(tool_display_name("my-custom-tool"), "my-custom-tool");
    }

    // ── looks_like_commit ─────────────────────────────────────────────────

    #[test]
    fn test_looks_like_commit_valid() {
        assert!(looks_like_commit("abc1234"));
        assert!(looks_like_commit("abc1234def567890"));
    }

    #[test]
    fn test_looks_like_commit_too_short() {
        assert!(!looks_like_commit("abc12"));
    }

    #[test]
    fn test_looks_like_commit_non_hex() {
        assert!(!looks_like_commit("unknown_commit"));
    }

    // ── render ────────────────────────────────────────────────────────────

    #[test]
    fn test_render_contains_header_and_details() {
        let e = entry("implement jwt validation", "claude-code", None, &["Edit"], &["src/auth.rs"], "done");
        let ctx = make_ctx(vec![], vec!["src/auth.rs"]);
        let scope = ExtractionScope::Uncommitted;

        let out = render(&[], &[&e], &[], &ctx, &scope);

        assert!(out.contains("## 🤖 Prompt History"));
        assert!(out.contains("<details>"));
        assert!(out.contains("</details>"));
    }

    #[test]
    fn test_render_shows_prompt_count() {
        let e = entry("fix bug", "claude-code", None, &["Edit"], &[], "fixed");
        let ctx = make_ctx(vec![], vec![]);
        let scope = ExtractionScope::Uncommitted;

        let out = render(&[], &[&e], &[], &ctx, &scope);
        assert!(out.contains("1 prompt over"));
    }

    #[test]
    fn test_render_shows_categories() {
        let inv = entry("explain auth", "claude-code", None, &["Read"], &[], "understood");
        let sol = entry("fix bug", "claude-code", None, &["Edit"], &[], "fixed");
        let ctx = make_ctx(vec![], vec![]);
        let scope = ExtractionScope::Uncommitted;

        let out = render(&[&inv], &[&sol], &[], &ctx, &scope);

        assert!(out.contains("### 🔍 Investigation"));
        assert!(out.contains("### 🔧 Solution"));
        assert!(!out.contains("### ✅ Testing")); // empty group omitted
    }

    #[test]
    fn test_render_includes_commit_refs() {
        let e = entry("fix bug", "claude-code", None, &["Edit"], &[], "done");
        let ctx = make_ctx(vec![make_commit("abc1234")], vec![]);
        let scope = ExtractionScope::Uncommitted;

        let out = render(&[], &[&e], &[], &ctx, &scope);
        assert!(out.contains("`abc1234`"));
    }

    #[test]
    fn test_render_shows_branch_from_scope() {
        let e = entry("fix bug", "claude-code", None, &["Edit"], &[], "done");
        let ctx = make_ctx(vec![], vec![]);
        let scope = ExtractionScope::BranchLifetime {
            branch: "feature/auth-fix".to_string(),
            since_commit: "abc1234567".to_string(),
        };

        let out = render(&[], &[&e], &[], &ctx, &scope);
        assert!(out.contains("`feature/auth-fix`"));
    }

    #[test]
    fn test_render_shows_model_in_tool_line() {
        let e = entry("fix bug", "claude-code", Some("claude-sonnet-4-5"), &["Edit"], &[], "done");
        let ctx = make_ctx(vec![], vec![]);
        let scope = ExtractionScope::Uncommitted;

        let out = render(&[], &[&e], &[], &ctx, &scope);
        assert!(out.contains("claude-sonnet-4-5"));
    }

    #[test]
    fn test_render_empty_sections_omitted() {
        let e = entry("fix bug", "claude-code", None, &["Edit"], &[], "done");
        let ctx = make_ctx(vec![], vec![]);
        let scope = ExtractionScope::Uncommitted;

        // All entries in Solution — Investigation and Testing should not appear
        let out = render(&[], &[&e], &[], &ctx, &scope);
        assert!(!out.contains("Investigation"));
        assert!(!out.contains("Testing"));
    }

    #[test]
    fn test_render_summary_line() {
        let inv = entry("explain", "claude-code", None, &["Read"], &[], "");
        let sol = entry("fix", "claude-code", None, &["Edit"], &[], "");
        let tst = entry("test", "claude-code", None, &["Bash"], &[], "");
        let ctx = make_ctx(vec![], vec![]);
        let scope = ExtractionScope::Uncommitted;

        let out = render(&[&inv], &[&sol], &[&tst], &ctx, &scope);
        assert!(out.contains("3 prompts"));
        assert!(out.contains("1 investigation, 1 solution, 1 testing"));
    }
}
