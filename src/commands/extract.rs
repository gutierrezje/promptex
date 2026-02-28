use anyhow::Result;

pub fn execute(
    _uncommitted: bool,
    _commits: Option<usize>,
    _since_commit: Option<String>,
    _branch_lifetime: bool,
    _write: Option<Option<String>>,
) -> Result<()> {
    // TODO (Phase 3+): Git analysis → scope → correlation → curation → output
    println!("pmtx extract: not yet implemented");
    Ok(())
}
