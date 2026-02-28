//! Journal reader for loading prompt history

use super::entry::JournalEntry;
use crate::project_id;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Load all journal entries for a project
///
/// Returns entries in chronological order (oldest first, as written).
/// Empty journal returns Ok(vec![]).
pub fn load_journal(project_id: &str) -> Result<Vec<JournalEntry>> {
    let journal_path = project_id::get_journal_path(project_id)?;

    // If journal doesn't exist yet, return empty vec (not an error)
    if !journal_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&journal_path).context("Failed to open journal file")?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.context("Failed to read journal line")?;

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Parse JSON line
        let entry = JournalEntry::from_json_line(&line).with_context(|| {
            format!("Failed to parse journal entry at line {}", line_num + 1)
        })?;

        entries.push(entry);
    }

    Ok(entries)
}

/// Load journal entries filtered by branch
pub fn load_journal_for_branch(project_id: &str, branch: &str) -> Result<Vec<JournalEntry>> {
    let all_entries = load_journal(project_id)?;

    Ok(all_entries
        .into_iter()
        .filter(|e| e.branch == branch)
        .collect())
}

/// Count total entries in journal (fast - doesn't parse)
pub fn count_entries(project_id: &str) -> Result<usize> {
    let journal_path = project_id::get_journal_path(project_id)?;

    if !journal_path.exists() {
        return Ok(0);
    }

    let file = File::open(&journal_path)?;
    let reader = BufReader::new(file);

    Ok(reader.lines().filter_map(Result::ok).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::entry::JournalEntry;
    use crate::journal::writer::append_entries;
    use tempfile::TempDir;

    fn make_entry(branch: &str, prompt: &str) -> JournalEntry {
        JournalEntry::new(
            branch.to_string(),
            "abc123".to_string(),
            prompt.to_string(),
            vec!["src/lib.rs".to_string()],
            vec!["Read".to_string()],
            "test outcome".to_string(),
            "claude-code".to_string(),
            None,
        )
    }

    #[test]
    fn test_load_empty_journal() {
        let dir = TempDir::new().unwrap();
        let _guard = crate::project_id::set_test_home(dir.path());

        // Journal file does not exist yet — should return empty vec, not an error
        let entries = load_journal("test-empty").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_journal_with_entries() {
        let dir = TempDir::new().unwrap();
        let _guard = crate::project_id::set_test_home(dir.path());

        let written = vec![
            make_entry("main", "first prompt"),
            make_entry("main", "second prompt"),
            make_entry("feature/x", "third prompt"),
        ];
        append_entries("test-load", &written).unwrap();

        let loaded = load_journal("test-load").unwrap();
        assert_eq!(loaded.len(), 3);
        // Verify chronological order is preserved
        assert_eq!(loaded[0].prompt, "first prompt");
        assert_eq!(loaded[1].prompt, "second prompt");
        assert_eq!(loaded[2].prompt, "third prompt");
        // Verify field parsing
        assert_eq!(loaded[0].branch, "main");
        assert_eq!(loaded[2].branch, "feature/x");
    }

    #[test]
    fn test_filter_by_branch() {
        let dir = TempDir::new().unwrap();
        let _guard = crate::project_id::set_test_home(dir.path());

        let entries = vec![
            make_entry("feature/auth", "auth prompt 1"),
            make_entry("main", "main prompt"),
            make_entry("feature/auth", "auth prompt 2"),
        ];
        append_entries("test-filter", &entries).unwrap();

        let auth_entries = load_journal_for_branch("test-filter", "feature/auth").unwrap();
        assert_eq!(auth_entries.len(), 2);
        assert_eq!(auth_entries[0].prompt, "auth prompt 1");
        assert_eq!(auth_entries[1].prompt, "auth prompt 2");
    }
}
