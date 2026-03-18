use chrono::Utc;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::output::json_format::ExtractionReport;
use crate::output::markdown_format;
use crate::project_id;

pub fn execute(
    file: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    date: bool,
    stdout: bool,
) -> Result<()> {
    if date {
        let now = Utc::now();
        println!("{}", now.format("%Y%m%d-%H%M"));
        return Ok(());
    }
    let input = match file {
        Some(path) => fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?,
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read from stdin")?;
            buffer
        }
    };

    let report: ExtractionReport =
        serde_json::from_str(&input).context("Failed to parse JSON input into ExtractionReport")?;

    let out_md = markdown_format::render_markdown(&report);

    if stdout {
        println!("{out_md}");
        return Ok(());
    }

    let dir = match out_dir {
        Some(d) => d,
        None => {
            let cwd = env::current_dir().context("Failed to get current directory")?;
            let id = project_id::get_project_id(&cwd)?;
            let pd = project_id::get_project_dir(&id)?;
            fs::create_dir_all(&pd)
                .with_context(|| format!("Failed to create project directory: {}", pd.display()))?;
            pd
        }
    };

    let now = Utc::now();
    let filename = format!("PROMPTS-{}.md", now.format("%Y%m%d-%H%M"));
    let out_path = dir.join(filename);

    fs::write(&out_path, out_md)
        .with_context(|| format!("Failed to write to file: {}", out_path.display()))?;

    println!("{}", out_path.display());

    Ok(())
}
