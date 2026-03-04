use anyhow::Result;
use chrono::{DateTime, Utc};
use std::env;

use crate::analysis::git;
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

    let display_dir = {
        let home = dirs::home_dir();
        let path_str = project_dir.to_string_lossy().to_string();
        if let Some(h) = home {
            path_str.replacen(&h.to_string_lossy().to_string(), "~", 1)
        } else {
            path_str
        }
    };

    println!("Project: {id}");
    println!("Branch:  {branch_label}");
    println!("Storage: {display_dir}/");

    let extractions: Vec<_> = std::fs::read_dir(&project_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("PROMPTS-"))
                .collect()
        })
        .unwrap_or_default();

    if !extractions.is_empty() {
        println!();
        println!("Extractions: {}", extractions.len());
        let last = extractions
            .iter()
            .filter_map(|e| e.metadata().ok()?.modified().ok())
            .max()
            .map(DateTime::<Utc>::from);
        if let Some(ts) = last {
            println!("Last:        {}", format_relative(ts));
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
