//! Journal entry structure shared across all extractors.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single journal entry representing one prompt and its context.
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

    /// Which AI tool was used (e.g., "claude-code", "codex")
    pub tool: String,

    /// Model identifier if known (e.g., "claude-sonnet-4-6")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// For short replies (< 8 words): the preceding assistant turn that prompted
    /// this response. Gives the LLM categorizer enough context to interpret
    /// bare confirmations like "yes" or "go ahead".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub assistant_context: Option<String>,
}

impl JournalEntry {
    #[allow(clippy::too_many_arguments)]
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
            assistant_context: None,
        }
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
            Some("claude-sonnet-4-6".to_string()),
        );

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: JournalEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.branch, parsed.branch);
        assert_eq!(entry.prompt, parsed.prompt);
        assert_eq!(entry.tool, parsed.tool);
    }
}
