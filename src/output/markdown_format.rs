use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;

use crate::output::json_format::ExtractionReport;
use crate::prompt::PromptEntry;

pub fn render_markdown(report: &ExtractionReport) -> String {
    let mut out = String::new();

    let mut entries: Vec<&PromptEntry> = report.entries.iter().collect();

    if entries.is_empty() {
        return "No prompts found for this scope.".to_string();
    }

    // Sort entries by timestamp
    entries.sort_by_key(|e| e.timestamp);

    // Compute basic stats
    let duration = report.until.signed_duration_since(report.since);
    let seconds = duration.num_seconds().max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    let duration_str = if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        "< 1m".to_string()
    };

    let total_prompts = entries.len();

    writeln!(out, "## Prompt History\n").unwrap();
    writeln!(out, "<details>").unwrap();
    writeln!(
        out,
        "<summary>{} prompts over {}</summary>\n",
        total_prompts, duration_str
    )
    .unwrap();

    // Session Details
    writeln!(out, "**Session Details**").unwrap();

    // Tools distribution
    let mut tool_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut tool_models: BTreeMap<&str, HashSet<&str>> = BTreeMap::new();
    let mut unique_models: HashSet<&str> = HashSet::new();

    for e in &entries {
        *tool_counts.entry(&e.tool).or_insert(0) += 1;
        if let Some(m) = &e.model {
            tool_models.entry(&e.tool).or_default().insert(m);
            unique_models.insert(m);
        }
    }

    let mut rules_tool_parts = Vec::new();
    for (tool, count) in &tool_counts {
        if let Some(models) = tool_models.get(tool) {
            let mut ma: Vec<_> = models.iter().copied().collect();
            ma.sort();
            rules_tool_parts.push(format!("{} ({}) - {} prompts", tool, ma.join(", "), count));
        } else {
            rules_tool_parts.push(format!("{} - {} prompts", tool, count));
        }
    }
    writeln!(out, "- Tools: {}", rules_tool_parts.join(", ")).unwrap();

    // Models
    if !unique_models.is_empty() {
        let mut ma: Vec<_> = unique_models.into_iter().collect();
        ma.sort();
        if ma.len() > 8 {
            let overflow = ma.len() - 8;
            ma.truncate(8);
            writeln!(
                out,
                "- Models: {}, +{} more",
                ma.iter()
                    .map(|s| format!("`{}`", s))
                    .collect::<Vec<_>>()
                    .join(", "),
                overflow
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "- Models: {}",
                ma.iter()
                    .map(|s| format!("`{}`", s))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }
    }

    let mut unique_branches: HashSet<&str> = HashSet::new();
    for e in &entries {
        unique_branches.insert(&e.branch);
    }
    let mut branches: Vec<_> = unique_branches.into_iter().collect();
    branches.sort();
    if !branches.is_empty() {
        writeln!(
            out,
            "- Branch: {}",
            branches
                .iter()
                .map(|b| format!("`{}`", b))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }

    writeln!(
        out,
        "- Time range: {} - {} (UTC)",
        report.since.format("%Y-%m-%d %H:%M"),
        report.until.format("%Y-%m-%d %H:%M")
    )
    .unwrap();

    if !report.commits.is_empty() {
        let commit_hashes: Vec<_> = report
            .commits
            .iter()
            .map(|c| format!("`{}`", c.short_hash))
            .collect();
        writeln!(
            out,
            "- Commits: {} ({} commits)",
            commit_hashes.join(", "),
            report.commits.len()
        )
        .unwrap();
    }

    let mut all_files = Vec::new();
    for e in &entries {
        for f in &e.files_touched {
            if !all_files.contains(f) {
                all_files.push(f.clone());
            }
        }
    }
    if !all_files.is_empty() {
        if all_files.len() > 8 {
            let overflow = all_files.len() - 8;
            let subset: Vec<_> = all_files
                .into_iter()
                .take(8)
                .map(|f| format!("`{}`", f))
                .collect();
            writeln!(
                out,
                "- Modified files: {}, +{} more",
                subset.join(", "),
                overflow
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "- Modified files: {}",
                all_files
                    .iter()
                    .map(|f| format!("`{}`", f))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }
    }

    writeln!(out, "\n---").unwrap();

    // Grouping
    let mut grouped: BTreeMap<String, Vec<&PromptEntry>> = BTreeMap::new();
    let mut inv_count = 0;
    let mut sol_count = 0;
    let mut tst_count = 0;

    for e in &entries {
        let cat = e.category.as_deref().unwrap_or("Uncategorized").to_string();
        grouped.entry(cat.clone()).or_default().push(*e);

        match cat.as_str() {
            "Investigation" => inv_count += 1,
            "Solution" => sol_count += 1,
            "Testing" => tst_count += 1,
            _ => {}
        }
    }

    // Ordered categories
    let order = vec!["Investigation", "Solution", "Testing", "Uncategorized"];
    for cat in order {
        if let Some(cat_entries) = grouped.get(cat) {
            writeln!(out, "\n### {}", cat).unwrap();

            for e in cat_entries {
                writeln!(out).unwrap();
                let time = e.timestamp.format("%H:%M");
                if let Some(m) = &e.model {
                    writeln!(out, "**[{}] ({} · {})**", time, e.tool, m).unwrap();
                } else {
                    writeln!(out, "**[{}] ({})**", time, e.tool).unwrap();
                }

                if contains_markdown_or_json(&e.prompt) {
                    let backticks = longest_backtick_sequence(&e.prompt);
                    let mut fence = String::new();
                    for _ in 0..std::cmp::max(4, backticks + 1) {
                        fence.push('`');
                    }
                    writeln!(out, "{}text\n{}\n{}", fence, e.prompt, fence).unwrap();
                } else {
                    for line in e.prompt.lines() {
                        writeln!(out, "> {}", line).unwrap();
                    }
                }

                let mut metadata = Vec::new();

                if let Some(ctx) = &e.assistant_context {
                    if let Some(question) = extract_question(ctx) {
                        metadata.push(format!("→ Re: *\"{}\"*", question));
                    }
                }

                if !e.tool_calls.is_empty() {
                    let mut tcs = e.tool_calls.clone();
                    tcs.dedup();
                    metadata.push(format!(
                        "→ Tools: {}",
                        tcs.iter()
                            .map(|t| format!("`{}`", t))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                if !e.files_touched.is_empty() {
                    metadata.push(format!(
                        "→ Files: {}",
                        e.files_touched
                            .iter()
                            .map(|f| format!("`{}`", f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                if !e.commit.is_empty() && e.commit.len() >= 7 {
                    metadata.push(format!("→ Commit: `{}`", &e.commit[..7]));
                }

                if !metadata.is_empty() {
                    writeln!(out).unwrap();
                    for m in metadata {
                        writeln!(out, "{}  ", m).unwrap();
                    }
                }
            }

            writeln!(out, "\n---").unwrap();
        }
    }

    writeln!(
        out,
        "\n**Summary:** {} prompts ({} investigation, {} solution, {} testing) · {} tools",
        total_prompts,
        inv_count,
        sol_count,
        tst_count,
        tool_counts.len()
    )
    .unwrap();
    writeln!(out, "\n</details>\n\n---").unwrap();
    writeln!(
        out,
        "\n*Generated with [PromptEx](https://github.com/gutierrezje/promptex)*"
    )
    .unwrap();

    out
}

fn contains_markdown_or_json(text: &str) -> bool {
    text.contains("```")
        || text.contains("\n#")
        || text.starts_with('#')
        || text.contains("\n> ")
        || text.starts_with("> ")
        || text.contains("\n- ")
        || text.starts_with("- ")
        || text.contains("\n* ")
        || text.starts_with("* ")
        || (text.contains('{') && text.contains('}') && text.contains('"'))
}

fn longest_backtick_sequence(text: &str) -> usize {
    let mut max_len = 0;
    let mut current_len = 0;
    for c in text.chars() {
        if c == '`' {
            current_len += 1;
            if current_len > max_len {
                max_len = current_len;
            }
        } else {
            current_len = 0;
        }
    }
    max_len
}

fn extract_question(ctx: &str) -> Option<String> {
    let lines: Vec<&str> = ctx
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.chars().all(|c| c == '─' || c == '-'))
        .collect();

    if let Some(last_line) = lines.last() {
        if last_line.contains('?') {
            let sentences: Vec<&str> = last_line.split(". ").collect();
            if let Some(last_sentence) = sentences.last() {
                if last_sentence.contains('?') {
                    let sanitized = last_sentence
                        .replace("`", "")
                        .replace("**", "")
                        .replace("_", "")
                        .replace(">", "")
                        .replace("- ", "")
                        .replace("* ", "");
                    let sanitized = sanitized.trim();
                    if sanitized.len() > 120 {
                        return Some(format!("{}...", &sanitized[..117]));
                    }
                    if !sanitized.is_empty() && sanitized.len() > 5 {
                        return Some(sanitized.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn render_markdown_escapes_backticks() {
        let since = Utc.with_ymd_and_hms(2026, 3, 1, 10, 0, 0).unwrap();

        let mut entry = PromptEntry::new(
            "main".to_string(),
            "".to_string(),
            "```rust\nfn test() {}\n```".to_string(),
            vec![],
            vec![],
            "codex".to_string(),
            None,
        );
        entry.category = Some("Solution".to_string());

        let report = ExtractionReport {
            scope: "uncommitted".to_string(),
            since,
            until: since,
            commits: vec![],
            scope_files: vec![],
            entries: vec![entry],
            warnings: vec![],
        };

        let md = render_markdown(&report);
        assert!(md.contains("````text"));
    }
}
