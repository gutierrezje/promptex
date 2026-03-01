use anyhow::{Context, Result};

use crate::commands::status::format_relative;
use crate::journal;
use crate::ProjectsAction;

pub fn execute(action: ProjectsAction) -> Result<()> {
    match action {
        ProjectsAction::List => list(),
        ProjectsAction::Remove { project_id } => remove(&project_id),
    }
}

fn list() -> Result<()> {
    let base = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".promptex")
        .join("projects");

    if !base.exists() {
        println!("No projects found. Run pmtx record or pmtx extract in a git repository.");
        return Ok(());
    }

    let mut projects: Vec<(String, usize, Option<chrono::DateTime<chrono::Utc>>)> = Vec::new();

    for entry in std::fs::read_dir(&base).context("Failed to read projects directory")? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            continue;
        }

        let count = journal::count_entries(&id).unwrap_or(0);
        let last_ts = journal::load_journal(&id)
            .ok()
            .and_then(|entries| entries.into_iter().last().map(|e| e.timestamp));

        projects.push((id, count, last_ts));
    }

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    // Sort: most recent first; projects with no entries go to the bottom
    projects.sort_by(|a, b| match (b.2, a.2) {
        (Some(bt), Some(at)) => bt.cmp(&at),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });

    let home = dirs::home_dir();
    for (id, count, last_ts) in &projects {
        println!("{id}");
        let dir = crate::project_id::get_project_dir(id).unwrap();
        let dir_str = {
            let s = dir.to_string_lossy().to_string();
            if let Some(ref h) = home {
                s.replacen(&h.to_string_lossy().to_string(), "~", 1)
            } else {
                s
            }
        };
        println!("  Journal: {dir_str}/");
        println!("  Prompts: {count}");
        if let Some(ts) = last_ts {
            println!("  Last entry: {}", format_relative(*ts));
        } else {
            println!("  Last entry: none");
        }
        println!();
    }

    Ok(())
}

fn remove(project_id: &str) -> Result<()> {
    let dir = crate::project_id::get_project_dir(project_id)?;

    if !dir.exists() {
        anyhow::bail!("Project '{project_id}' not found at {}", dir.display());
    }

    std::fs::remove_dir_all(&dir)
        .with_context(|| format!("Failed to remove {}", dir.display()))?;

    println!("Removed project '{project_id}'.");
    Ok(())
}
