use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use crate::commands::status::format_relative;
use crate::ProjectsAction;

pub fn execute(action: ProjectsAction) -> Result<()> {
    match action {
        ProjectsAction::List => list(),
        ProjectsAction::Remove { project_id } => remove(&project_id),
    }
}

struct ProjectInfo {
    id: String,
    last_ts: Option<chrono::DateTime<chrono::Utc>>,
    extractions: usize,
}

fn load_sorted_projects() -> Result<Vec<ProjectInfo>> {
    let base = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".promptex")
        .join("projects");

    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut projects: Vec<ProjectInfo> = Vec::new();

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

        let last_ts = std::fs::read_dir(&path).ok().and_then(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("PROMPTS-"))
                .filter_map(|e| e.metadata().ok()?.modified().ok())
                .max()
                .map(DateTime::<Utc>::from)
        });
        let extractions = std::fs::read_dir(&path)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().starts_with("PROMPTS-"))
                    .count()
            })
            .unwrap_or(0);

        projects.push(ProjectInfo {
            id,
            last_ts,
            extractions,
        });
    }

    projects.sort_by(|a, b| match (b.last_ts, a.last_ts) {
        (Some(bt), Some(at)) => bt.cmp(&at),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    Ok(projects)
}

fn list() -> Result<()> {
    let projects = load_sorted_projects()?;

    if projects.is_empty() {
        println!("No projects found. Run `pmtx extract` in a git repository to create one.");
        return Ok(());
    }

    let home = dirs::home_dir();
    for (i, p) in projects.iter().enumerate() {
        println!("{}  {}", i + 1, p.id);
        let dir = crate::project_id::get_project_dir(&p.id).unwrap();
        let dir_str = {
            let s = dir.to_string_lossy().to_string();
            if let Some(ref h) = home {
                s.replacen(&h.to_string_lossy().to_string(), "~", 1)
            } else {
                s
            }
        };
        println!("   Storage: {dir_str}/");
        println!("   Extractions: {}", p.extractions);
        if let Some(ts) = p.last_ts {
            println!("   Last entry: {}", format_relative(ts));
        } else {
            println!("   Last entry: none");
        }
        println!();
    }

    Ok(())
}

fn remove(project_id: &str) -> Result<()> {
    let resolved_id = if let Ok(n) = project_id.parse::<usize>() {
        let projects = load_sorted_projects()?;
        projects
            .into_iter()
            .nth(n.saturating_sub(1))
            .map(|p| p.id)
            .with_context(|| {
                format!("No project at index {n} — run `pmtx projects list` to see options")
            })?
    } else {
        project_id.to_string()
    };

    let dir = crate::project_id::get_project_dir(&resolved_id)?;

    if !dir.exists() {
        anyhow::bail!("Project '{resolved_id}' not found at {}", dir.display());
    }

    std::fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove {}", dir.display()))?;

    println!("Removed project '{resolved_id}'.");
    Ok(())
}
