//! Journal entry structure for prompt logging

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single journal entry representing one prompt and its context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// When this prompt was issued (ISO-8601)
    pub timestamp: DateTime<Utc>,

    /// Git branch at time of prompt
    pub branch: String,

    /// Git commit hash at time of prompt
    pub commit: String,

    /// The prompt text (redacted for privacy)
    pub prompt: String,

    /// Files touched by this prompt's tool calls
    pub files_touched: Vec<String>,

    /// Tool calls made (e.g., ["Edit", "Bash", "Read"])
    pub tool_calls: Vec<String>,

    /// Brief description of what happened / was accomplished
    pub outcome: String,

    /// Which AI tool was used (e.g., "claude-code", "cursor")
    pub tool: String,

    /// Model identifier if known (e.g., "claude-sonnet-4.5")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl JournalEntry {
    /// Create a new journal entry with current timestamp
    pub fn new(
        branch: String,
        commit: String,
        prompt: String,
        files_touched: Vec<String>,
        tool_calls: Vec<String>,
        outcome: String,
        tool: String,
        model: Option<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            branch,
            commit,
            prompt,
            files_touched,
            tool_calls,
            outcome,
            tool,
            model,
        }
    }

    /// Serialize entry to JSON line (for journal.jsonl)
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize entry from JSON line
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_entry_roundtrip() {
        let entry = JournalEntry::new(
            "feature/auth".to_string(),
            "abc123".to_string(),
            "implement JWT validation".to_string(),
            vec!["src/auth.rs".to_string()],
            vec!["Edit".to_string()],
            "Added expiry check".to_string(),
            "claude-code".to_string(),
            Some("claude-sonnet-4.5".to_string()),
        );

        let json = entry.to_json_line().unwrap();
        let parsed = JournalEntry::from_json_line(&json).unwrap();

        assert_eq!(entry.branch, parsed.branch);
        assert_eq!(entry.prompt, parsed.prompt);
        assert_eq!(entry.tool, parsed.tool);
    }
}
