use anyhow::Result;
use console::style;
use std::path::PathBuf;
use crate::config::Config;
use crate::services::github::{IssueUrl, clone_repo};

pub async fn execute(url: &str, directory: Option<&str>) -> Result<()> {
    let parsed = IssueUrl::parse(url)?;
    let config = Config::load()?;
    let destination = directory.map(PathBuf::from);
    let clone_path = clone_repo(
        &parsed.owner,
        &parsed.repo,
        destination.as_deref(),
        config.defaults.shallow_clone,
    )?;

    println!("{}", style("🎯 Fetching issue and generating context pack...").cyan().bold());
    println!();
    println!("  URL: {}", url);
    println!("  Repository: {}/{}", parsed.owner, parsed.repo);
    println!("  Issue: #{}", parsed.issue_number);
    println!(
        "  Clone directory: {}",
        directory.unwrap_or(parsed.repo.as_str())
    );
    println!("  Cloned to: {}", clone_path.display());
    println!();
    println!(
        "{}",
        style("✓ Repository cloned. Context generation implementation is next.").green()
    );

    Ok(())
}
