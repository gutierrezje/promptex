//! Journal writer for append-only logging

use super::entry::JournalEntry;
use crate::project_id;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;

/// Append a journal entry to the project's journal.jsonl
///
/// This is the core journaling operation - entries are never modified,
/// only appended. This creates an immutable audit trail.
pub fn append_entry(project_id: &str, entry: &JournalEntry) -> Result<()> {
    // Ensure project directory exists
    project_id::ensure_project_dir(project_id)?;

    let journal_path = project_id::get_journal_path(project_id)?;

    // Open in append mode (create if doesn't exist)
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&journal_path)
        .context("Failed to open journal file")?;

    // Serialize entry to JSON line
    let json_line = entry
        .to_json_line()
        .context("Failed to serialize journal entry")?;

    // Write line + newline
    writeln!(file, "{}", json_line).context("Failed to write journal entry")?;

    Ok(())
}

/// Append multiple entries at once (batch operation)
pub fn append_entries(project_id: &str, entries: &[JournalEntry]) -> Result<()> {
    project_id::ensure_project_dir(project_id)?;

    let journal_path = project_id::get_journal_path(project_id)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&journal_path)
        .context("Failed to open journal file")?;

    for entry in entries {
        let json_line = entry
            .to_json_line()
            .context("Failed to serialize journal entry")?;
        writeln!(file, "{}", json_line).context("Failed to write journal entry")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(branch: &str, prompt: &str) -> JournalEntry {
        JournalEntry::new(
            branch.to_string(),
            "abc123".to_string(),
            prompt.to_string(),
            vec!["src/main.rs".to_string()],
            vec!["Edit".to_string()],
            "test outcome".to_string(),
            "claude-code".to_string(),
            None,
        )
    }

    #[test]
    fn test_append_entry() {
        let dir = TempDir::new().unwrap();
        let _guard = crate::project_id::set_test_home(dir.path());

        let entry = make_entry("main", "implement auth");
        append_entry("test-single", &entry).unwrap();

        let journal_path = dir.path().join("projects/test-single/journal.jsonl");
        assert!(journal_path.exists(), "journal.jsonl should be created");

        let content = std::fs::read_to_string(&journal_path).unwrap();
        let parsed: JournalEntry = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed.branch, "main");
        assert_eq!(parsed.prompt, "implement auth");
    }

    #[test]
    fn test_append_multiple_entries() {
        let dir = TempDir::new().unwrap();
        let _guard = crate::project_id::set_test_home(dir.path());

        let entries = vec![
            make_entry("feature/a", "first"),
            make_entry("feature/b", "second"),
            make_entry("main", "third"),
        ];
        append_entries("test-multi", &entries).unwrap();

        let journal_path = dir.path().join("projects/test-multi/journal.jsonl");
        let content = std::fs::read_to_string(&journal_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 3, "should have 3 lines");
        let e0: JournalEntry = serde_json::from_str(lines[0]).unwrap();
        let e1: JournalEntry = serde_json::from_str(lines[1]).unwrap();
        let e2: JournalEntry = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(e0.branch, "feature/a");
        assert_eq!(e1.branch, "feature/b");
        assert_eq!(e2.branch, "main");
    }
}
