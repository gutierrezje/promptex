use anyhow::Result;
use std::path::Path;
use serde::Serialize;

/// Generate all context pack files in .issuance/ directory
pub fn generate_context_pack(
    output_dir: &Path,
    issue_data: &impl Serialize,
    codemap_data: &impl Serialize,
    signals_data: &impl Serialize,
    rules_data: &impl Serialize,
    handoff_data: &impl Serialize,
) -> Result<()> {
    // TODO: Implement template rendering and file generation
    todo!("Generate context pack files")
}

/// Generate metadata.json with session information
pub fn generate_metadata(
    output_dir: &Path,
    issue_url: &str,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<()> {
    // TODO: Implement metadata generation
    todo!("Generate metadata.json")
}
