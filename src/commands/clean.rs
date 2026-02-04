use anyhow::{Context, Result};
use console::style;
use std::fs;
use std::path::Path;

pub fn execute() -> Result<()> {
    let issuance_dir = Path::new(".issuance");

    if !issuance_dir.exists() {
        println!("{}", style("✓ .issuance/ folder doesn't exist (nothing to clean)").dim());
        return Ok(());
    }

    // Remove the directory
    fs::remove_dir_all(issuance_dir)
        .context("Failed to remove .issuance/ folder")?;

    println!("{}", style("✓ Removed .issuance/").green());

    Ok(())
}
