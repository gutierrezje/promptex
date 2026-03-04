//! Prompt entry structure shared across all extractors.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single extracted prompt and its surrounding context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEntry {
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

    /// Which AI tool was used (e.g., "claude-code", "codex")
    pub tool: String,

    /// Model identifier if known (e.g., "claude-sonnet-4-6")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// The tail of the most recent preceding assistant turn. Captured unconditionally
    /// so the skill can use it for categorization context — especially useful for
    /// short confirmations ("yes", "go ahead") or hybrid messages that begin with
    /// approval before adding new context ("yes fix that. also...").
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub assistant_context: Option<String>,
}

impl PromptEntry {
    pub fn new(
        branch: String,
        commit: String,
        prompt: String,
        files_touched: Vec<String>,
        tool_calls: Vec<String>,
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
    fn test_prompt_entry_roundtrip() {
        let entry = PromptEntry::new(
            "feature/auth".to_string(),
            "abc123".to_string(),
            "implement JWT validation".to_string(),
            vec!["src/auth.rs".to_string()],
            vec!["Edit".to_string()],
            "claude-code".to_string(),
            Some("claude-sonnet-4-6".to_string()),
        );

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: PromptEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.branch, parsed.branch);
        assert_eq!(entry.prompt, parsed.prompt);
        assert_eq!(entry.tool, parsed.tool);
    }
}
