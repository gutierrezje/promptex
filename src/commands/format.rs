use chrono::Utc;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::output::json_format::ExtractionReport;
use crate::output::markdown_format;

pub fn execute(file: Option<PathBuf>, out_dir: Option<PathBuf>, date: bool) -> Result<()> {
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

    if let Some(dir) = out_dir {
        let now = Utc::now();
        let filename = format!("PROMPTS-{}.md", now.format("%Y%m%d-%H%M"));
        let out_path = dir.join(filename);

        fs::write(&out_path, out_md)
            .with_context(|| format!("Failed to write to file: {}", out_path.display()))?;

        println!("{}", out_path.display());
    } else {
        println!("{out_md}");
    }

    Ok(())
}
