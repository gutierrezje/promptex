//! Curation filters — artifact gating and near-duplicate removal.
//!
//! Two passes are applied in order:
//!   1. `apply_artifact_filter` — drop entries that produced no observable work
//!   2. `remove_duplicates`     — collapse near-identical rephrases into one
//!
//! Both take ownership of the entry list and return a new filtered list.
//! Keeping the functions pure (no mutation) makes them easy to test and reason
//! about in the pipeline.

use std::collections::HashSet;

use crate::analysis::correlation::has_artifact;
use crate::journal::JournalEntry;

// ── Public API ─────────────────────────────────────────────────────────────────

/// Drop entries that have no concrete artifact — no file edits, no tool calls.
///
/// These are "thinking out loud" prompts: pure conversation turns where the
/// agent read nothing and wrote nothing. They add noise without adding context
/// for a PR reviewer.
pub fn apply_artifact_filter(entries: Vec<JournalEntry>) -> Vec<JournalEntry> {
    entries.into_iter().filter(|e| has_artifact(e)).collect()
}

/// Collapse near-identical prompts into one, keeping the most recent version.
///
/// Users often rephrase a prompt slightly after an unsatisfying response —
/// "fix the auth bug" → "fix the JWT expiry bug in auth.rs". Jaccard similarity
/// on word tokens catches these clusters. When a near-duplicate is found, the
/// *newer* entry replaces the older one (the refinement is preferred).
///
/// Threshold: 0.80 Jaccard similarity (word tokens, lowercased).
pub fn remove_duplicates(mut entries: Vec<JournalEntry>) -> Vec<JournalEntry> {
    // Sort ascending so we iterate oldest → newest.
    // When a duplicate is found we replace the older entry with the newer one.
    entries.sort_by_key(|e| e.timestamp);

    let mut kept: Vec<JournalEntry> = Vec::new();

    for candidate in entries {
        let dup_pos = kept.iter().position(|existing| {
            jaccard_similarity(&candidate.prompt, &existing.prompt) > DEDUP_THRESHOLD
        });

        match dup_pos {
            // Replace the older entry with the newer (more refined) version.
            Some(pos) => kept[pos] = candidate,
            // No duplicate found — keep as new entry.
            None => kept.push(candidate),
        }
    }

    kept
}

// ── Helpers ───────────────────────────────────────────────────────────────────

const DEDUP_THRESHOLD: f64 = 0.80;

/// Jaccard similarity between the word-token sets of two strings.
///
/// Tokens are split on non-alphanumeric characters and lowercased so that
/// "Fix Auth" and "fix auth" compare as identical.
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let tokens_a = word_tokens(a);
    let tokens_b = word_tokens(b);

    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 1.0;
    }

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn word_tokens(s: &str) -> HashSet<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn entry_at(prompt: &str, h: i64, tool_calls: &[&str], files: &[&str]) -> JournalEntry {
        JournalEntry {
            timestamp: Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap()
                + Duration::hours(h),
            branch: "feature/test".to_string(),
            commit: "abc1234".to_string(),
            prompt: prompt.to_string(),
            files_touched: files.iter().map(|s| s.to_string()).collect(),
            tool_calls: tool_calls.iter().map(|s| s.to_string()).collect(),
            outcome: String::new(),
            tool: "claude-code".to_string(),
            model: None,
        }
    }

    // ── apply_artifact_filter ─────────────────────────────────────────────

    #[test]
    fn test_artifact_filter_removes_empty_entries() {
        let entries = vec![entry_at("explain auth", 0, &[], &[])];
        let result = apply_artifact_filter(entries);
        assert!(result.is_empty());
    }

    #[test]
    fn test_artifact_filter_keeps_entry_with_tool_calls() {
        let entries = vec![entry_at("implement auth", 0, &["Edit"], &[])];
        let result = apply_artifact_filter(entries);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_artifact_filter_keeps_entry_with_files_touched() {
        let entries = vec![entry_at("look at auth.rs", 0, &[], &["src/auth.rs"])];
        let result = apply_artifact_filter(entries);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_artifact_filter_mixed() {
        let entries = vec![
            entry_at("thinking out loud", 0, &[], &[]),  // should be removed
            entry_at("implement auth", 1, &["Edit"], &[]), // should be kept
        ];
        let result = apply_artifact_filter(entries);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].prompt, "implement auth");
    }

    // ── remove_duplicates ─────────────────────────────────────────────────

    #[test]
    fn test_dedup_keeps_unique_prompts() {
        let entries = vec![
            entry_at("implement jwt validation", 0, &["Edit"], &[]),
            entry_at("run the tests", 1, &["Bash"], &[]),
        ];
        let result = remove_duplicates(entries);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_collapses_identical_prompts() {
        let entries = vec![
            entry_at("implement jwt validation", 0, &["Edit"], &[]),
            entry_at("implement jwt validation", 1, &["Edit"], &[]),
        ];
        let result = remove_duplicates(entries);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_keeps_newer_version() {
        // The second entry is the refined rephrasing — it should survive.
        let entries = vec![
            entry_at("fix auth bug", 0, &["Edit"], &[]),
            entry_at("fix auth bug in verify_token", 1, &["Edit"], &[]),
        ];
        // Jaccard: {"fix","auth","bug"} vs {"fix","auth","bug","in","verify_token"}
        // intersection=3, union=5, similarity=0.6 — NOT a duplicate
        // so both should be kept
        let result = remove_duplicates(entries);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_high_similarity_collapses() {
        // One extra filler word — should be treated as near-duplicate.
        // "implement jwt validation" vs "implement jwt validation please"
        // tokens_a=3, tokens_b=4, intersection=3, union=4, sim=0.75 — just under threshold.
        // Add one more overlap word to push over 0.80.
        // "implement the jwt validation" vs "implement the jwt validation now"
        // tokens_a={"implement","the","jwt","validation"}, tokens_b same + "now"
        // intersection=4, union=5, sim=0.8 → exactly at threshold (not strictly >)
        // "implement the jwt validation here" vs "implement the jwt validation here now"
        // tokens_a=5, tokens_b=6, intersection=5, union=6, sim=0.833 → duplicate
        let entries = vec![
            entry_at("implement the jwt validation here", 0, &["Edit"], &[]),
            entry_at("implement the jwt validation here now", 1, &["Edit"], &[]),
        ];
        let result = remove_duplicates(entries);
        assert_eq!(result.len(), 1);
        // The newer (t=1) should win
        assert!(result[0].prompt.contains("now"));
    }

    #[test]
    fn test_dedup_case_insensitive() {
        let entries = vec![
            entry_at("Implement JWT Validation", 0, &["Edit"], &[]),
            entry_at("implement jwt validation", 1, &["Edit"], &[]),
        ];
        let result = remove_duplicates(entries);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_dedup_empty_input() {
        let result = remove_duplicates(vec![]);
        assert!(result.is_empty());
    }

    // ── jaccard_similarity (via dedup behaviour) ──────────────────────────

    #[test]
    fn test_jaccard_identical_strings() {
        // Simulate via dedup
        let entries = vec![
            entry_at("exact same prompt text", 0, &["Edit"], &[]),
            entry_at("exact same prompt text", 1, &["Edit"], &[]),
        ];
        assert_eq!(remove_duplicates(entries).len(), 1);
    }

    #[test]
    fn test_jaccard_completely_different() {
        let entries = vec![
            entry_at("implement jwt validation", 0, &["Edit"], &[]),
            entry_at("run database migrations", 1, &["Bash"], &[]),
        ];
        assert_eq!(remove_duplicates(entries).len(), 2);
    }
}
