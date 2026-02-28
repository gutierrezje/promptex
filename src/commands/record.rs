use anyhow::Result;

pub fn execute(
    _prompt: &str,
    _files: Vec<String>,
    _tool_calls: Vec<String>,
    _outcome: &str,
    _tool: &str,
    _model: Option<String>,
) -> Result<()> {
    // TODO (Phase 4): Redact → build JournalEntry → append_entry
    println!("pmtx record: not yet implemented");
    Ok(())
}
