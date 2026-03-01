use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::env;

use crate::analysis::git;
use crate::journal;
use crate::project_id;

pub fn execute() -> Result<()> {
    let cwd = env::current_dir()?;

    let id = project_id::get_project_id(&cwd)?;
    let project_dir = project_id::get_project_dir(&id)?;

    let branch = git::current_branch().unwrap_or_else(|_| "unknown".to_string());
    let branch_label = if git::is_mainline_branch(&branch) {
        format!("{branch} (mainline)")
    } else {
        branch.clone()
    };

    // Abbreviate home directory as ~ for readability
    let display_dir = {
        let home = dirs::home_dir();
        let path_str = project_dir.to_string_lossy().to_string();
        if let Some(h) = home {
            let home_str = h.to_string_lossy().to_string();
            path_str.replacen(&home_str, "~", 1)
        } else {
            path_str
        }
    };

    println!("Project: {id}");
    println!("Branch:  {branch_label}");
    println!("Journal: {display_dir}/");
    println!();

    let entries = journal::load_journal(&id)?;
    let total = entries.len();
    println!("Prompts logged: {total}");

    if !entries.is_empty() {
        // Group by branch
        let mut branch_counts: HashMap<String, usize> = HashMap::new();
        for e in &entries {
            *branch_counts.entry(e.branch.clone()).or_insert(0) += 1;
        }

        // Sort branches by count descending, then alphabetically
        let mut sorted: Vec<(String, usize)> = branch_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        // Align counts under the branch column
        let label_width = sorted.iter().map(|(b, _)| b.len()).max().unwrap_or(0);
        for (b, count) in &sorted {
            println!("  - {b:<label_width$}  {count}");
        }

        println!();
        if let Some(last) = entries.last() {
            println!("Last entry: {}", format_relative(last.timestamp));
        }
    }

    Ok(())
}

pub(crate) fn format_relative(ts: DateTime<Utc>) -> String {
    let now = Utc::now();
    let secs = (now - ts).num_seconds().max(0);
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let m = secs / 60;
        format!("{m} minute{} ago", if m == 1 { "" } else { "s" })
    } else if secs < 86400 {
        let h = secs / 3600;
        format!("{h} hour{} ago", if h == 1 { "" } else { "s" })
    } else {
        let d = secs / 86400;
        format!("{d} day{} ago", if d == 1 { "" } else { "s" })
    }
}
