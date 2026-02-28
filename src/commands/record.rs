use anyhow::{Context, Result};
use crate::analysis::git;
use crate::curation::redact;
use crate::journal::{self, JournalEntry};
use crate::project_id;

pub fn execute(
    prompt: &str,
    files: Vec<String>,
    tool_calls: Vec<String>,
    outcome: &str,
    tool: &str,
    model: Option<String>,
) -> Result<()> {
    // Capture git context at the moment of recording
    let branch = git::current_branch()
        .context("pmtx record must be run inside a git repository")?;
    let commit = git::current_commit()?;

    // Redact before anything touches disk
    let (redacted_prompt, redactions) = redact::redact(prompt);
    if !redactions.is_empty() {
        eprintln!(
            "⚠  Redacted {} sensitive value(s) from prompt:",
            redactions.len()
        );
        for r in &redactions {
            eprintln!("   - {}", r.kind);
        }
    }

    let entry = JournalEntry::new(
        branch,
        commit,
        redacted_prompt,
        files,
        tool_calls,
        outcome.to_string(),
        tool.to_string(),
        model,
    );

    // Resolve project ID from git remote and write
    let cwd = std::env::current_dir().context("Could not determine current directory")?;
    let project_id = project_id::get_project_id(&cwd)
        .context("Could not determine project ID — is there a git remote configured?")?;

    journal::append_entry(&project_id, &entry)?;

    eprintln!("✓ Journaled to ~/.promptex/projects/{project_id}/journal.jsonl");
    Ok(())
}
