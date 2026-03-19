use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::path::Path;

use crate::prompt::PromptEntry;

/// Shared validator to enforce normalized extraction behavior.
pub(crate) fn assert_prompt_entry_contract(
    entry: &PromptEntry,
    _project_root: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) {
    assert!(
        !entry.prompt.trim().is_empty(),
        "prompt should not be empty"
    );

    assert!(
        entry.timestamp >= since && entry.timestamp <= until,
        "timestamp {} is outside window [{}, {}]",
        entry.timestamp,
        since,
        until
    );

    let allowed_labels = ["Read", "Write", "Bash", "Explore", "Skill"];
    let mut seen_tools: HashSet<&str> = HashSet::new();
    for tool in &entry.tool_calls {
        assert!(
            allowed_labels.contains(&tool.as_str()),
            "tool label '{}' is not canonical (Read, Write, Bash, Explore, Skill)",
            tool
        );
        assert!(
            seen_tools.insert(tool.as_str()),
            "duplicate tool call: {tool}"
        );
    }

    let mut seen_files: HashSet<&str> = HashSet::new();
    for file in &entry.files_touched {
        let path = Path::new(file);
        assert!(
            !path.is_absolute(),
            "file path should not be absolute: {file}"
        );
        assert!(
            !file.starts_with("~/"),
            "file path should not be home-relative: {file}"
        );
        assert!(
            !file.starts_with("../")
                && !file.contains("/../")
                && !file.ends_with("/..")
                && file != "..",
            "file path should not contain parent traversal: {file}"
        );
        assert!(
            seen_files.insert(file.as_str()),
            "duplicate file touched: {file}"
        );
    }
}

/// Assert contract for a batch of entries.
pub(crate) fn assert_entries_contract(
    entries: &[PromptEntry],
    project_root: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) {
    for entry in entries {
        assert_prompt_entry_contract(entry, project_root, since, until);
    }
}
