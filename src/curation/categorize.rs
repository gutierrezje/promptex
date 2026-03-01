//! Intent categorization for curated journal entries.
//!
//! Categorization uses a simple scoring heuristic that combines two signals:
//!   1. Tool calls (strong signal — what the agent actually did)
//!   2. Prompt keywords (weaker signal — what the user asked for)
//!
//! Scores are tallied per category and the highest wins.
//! In ties, Solution is the default (most common action type).

use crate::journal::JournalEntry;

// ── Intent ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Understanding existing code — reading, exploring, asking questions.
    Investigation,
    /// Writing or modifying code — implementing, fixing, refactoring.
    Solution,
    /// Verifying behavior — running tests, checking output, validating.
    Testing,
}

impl Intent {
    pub fn label(&self) -> &'static str {
        match self {
            Intent::Investigation => "Investigation",
            Intent::Solution => "Solution",
            Intent::Testing => "Testing",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Intent::Investigation => "🔍",
            Intent::Solution => "🔧",
            Intent::Testing => "✅",
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Categorize a single journal entry into an [`Intent`].
///
/// Tool calls are scored first because they reflect *what actually happened*,
/// not just what was requested. Keywords on the prompt and outcome text act as
/// tiebreakers or additional signal when tool call data is sparse.
pub fn categorize(entry: &JournalEntry) -> Intent {
    let mut inv = 0u32;  // investigation score
    let mut sol = 0u32;  // solution score
    let mut tst = 0u32;  // testing score

    // ── Tool call signals (strong) ─────────────────────────────────────────
    for call in &entry.tool_calls {
        match call.as_str() {
            // Mutations → Solution
            "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => sol += 3,
            // Read-only → Investigation
            "Read" | "Glob" | "Grep" | "LS" | "NotebookRead" => inv += 2,
            // Web reads → weak investigation signal
            "WebFetch" | "WebSearch" => inv += 1,
            // Bash most commonly runs test commands → slight testing lean
            "Bash" => tst += 1,
            _ => {}
        }
    }

    // ── Prompt keyword signals ─────────────────────────────────────────────
    let prompt = entry.prompt.to_lowercase();

    for kw in INVESTIGATION_KEYWORDS {
        if prompt.contains(kw) {
            inv += 2;
        }
    }
    for kw in SOLUTION_KEYWORDS {
        if prompt.contains(kw) {
            sol += 2;
        }
    }
    for kw in TESTING_KEYWORDS {
        if prompt.contains(kw) {
            tst += 2;
        }
    }

    // ── Outcome keyword signals (weak) ────────────────────────────────────
    let outcome = entry.outcome.to_lowercase();
    if outcome.contains("test") || outcome.contains("passing") || outcome.contains("verified") {
        tst += 1;
    }
    if outcome.contains("implement") || outcome.contains("added") || outcome.contains("created") {
        sol += 1;
    }
    if outcome.contains("identified") || outcome.contains("found") || outcome.contains("understand") {
        inv += 1;
    }

    // ── Decision — ties go to Solution (most common) ───────────────────────
    if tst > inv && tst > sol {
        Intent::Testing
    } else if inv > sol {
        Intent::Investigation
    } else {
        Intent::Solution
    }
}

// ── Keyword tables ─────────────────────────────────────────────────────────────

static INVESTIGATION_KEYWORDS: &[&str] = &[
    "understand", "explain", "show me", "how does", "what is", "where is",
    "what does", "why is", "how is", "describe", "look at", "find", "explore",
    "read", "search", "which", "what are",
];

static SOLUTION_KEYWORDS: &[&str] = &[
    "implement", "add", "fix", "write", "create", "update", "change",
    "refactor", "move", "rename", "delete", "remove", "make", "build",
    "modify", "edit", "replace", "extract", "migrate", "convert",
];

static TESTING_KEYWORDS: &[&str] = &[
    "test", "verify", "check", "validate", "run", "assert", "debug",
    "confirm", "ensure", "passing", "failing",
];

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn entry(prompt: &str, tool_calls: &[&str]) -> JournalEntry {
        JournalEntry {
            timestamp: Utc::now(),
            branch: "feature/test".to_string(),
            commit: "abc1234".to_string(),
            prompt: prompt.to_string(),
            files_touched: vec![],
            tool_calls: tool_calls.iter().map(|s| s.to_string()).collect(),
            outcome: String::new(),
            tool: "claude-code".to_string(),
            model: None,
            assistant_context: None,
        }
    }

    #[test]
    fn test_investigation_keyword_and_read_calls() {
        let e = entry("explain how the JWT validation works", &["Read", "Grep"]);
        assert_eq!(categorize(&e), Intent::Investigation);
    }

    #[test]
    fn test_solution_by_edit_call() {
        let e = entry("add expiry validation to verify_token", &["Edit", "Read"]);
        // Edit (+3) dominates Read (+2), plus "add" keyword
        assert_eq!(categorize(&e), Intent::Solution);
    }

    #[test]
    fn test_testing_by_keyword_and_bash() {
        let e = entry("run the tests to verify the fix", &["Bash"]);
        // "test" (+2) + "verify" (+2) + Bash (+1) = 5 testing vs 0 others
        assert_eq!(categorize(&e), Intent::Testing);
    }

    #[test]
    fn test_tool_calls_beat_keywords() {
        // Prompt says "explain" but agent actually did Edit — it was Solution
        let e = entry("explain and then fix the auth bug", &["Edit", "Write"]);
        // sol: "fix" (+2) + Edit (+3) + Write (+3) = 8 > inv: "explain" (+2) = 2
        assert_eq!(categorize(&e), Intent::Solution);
    }

    #[test]
    fn test_default_is_solution_on_tie() {
        // No tool calls, no recognizable keywords
        let e = entry("do the thing", &[]);
        assert_eq!(categorize(&e), Intent::Solution);
    }

    #[test]
    fn test_investigation_wins_with_only_reads() {
        let e = entry("look at the auth module", &["Read", "Glob", "Grep"]);
        // inv: "look at" (+2) + Read (+2) + Glob (+2) + Grep (+2) = 8
        assert_eq!(categorize(&e), Intent::Investigation);
    }

    #[test]
    fn test_testing_bash_keyword_combo() {
        let e = entry("check that all tests pass", &["Bash"]);
        // tst: "check" (+2) + "test" (+2) + Bash (+1) = 5
        assert_eq!(categorize(&e), Intent::Testing);
    }

    #[test]
    fn test_intent_labels() {
        assert_eq!(Intent::Investigation.label(), "Investigation");
        assert_eq!(Intent::Solution.label(), "Solution");
        assert_eq!(Intent::Testing.label(), "Testing");
    }

    #[test]
    fn test_intent_emojis() {
        assert_eq!(Intent::Investigation.emoji(), "🔍");
        assert_eq!(Intent::Solution.emoji(), "🔧");
        assert_eq!(Intent::Testing.emoji(), "✅");
    }
}
