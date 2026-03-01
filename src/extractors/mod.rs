//! Extractor detection and dispatch.
//!
//! `detect()` inspects the current environment and collects ALL available
//! extractors. Entries from every available source are merged and sorted by
//! timestamp so cross-tool sessions (e.g. Claude Code + Codex on the same
//! branch) appear together.
//!
//! ## Extractor support status
//! | Tool           | Status  | Notes                                           |
//! |----------------|---------|------------------------------------------------ |
//! | Claude Code    | ✅ Active | JSONL sessions at `~/.claude/projects/`        |
//! | Codex CLI/App  | ✅ Active | JSONL sessions at `~/.codex/sessions/`         |
//! | OpenCode       | ⏳ TODO  | Migrated to SQLite (v1.2+); needs rewrite       |
//! | Cursor         | ⏳ TODO  | Log format TBD                                  |
//! | GitHub Copilot | ⏳ TODO  | Log format TBD                                  |

pub mod claude_code;
pub mod codex;
pub mod opencode; // kept for future rewrite — not wired into detect()
pub mod traits;

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use crate::curation::redact::redact;
use crate::journal::JournalEntry;
use claude_code::ClaudeCodeExtractor;
use codex::CodexExtractor;
use traits::PromptExtractor;

type ExtractFn = Box<dyn Fn(DateTime<Utc>, DateTime<Utc>) -> Result<Vec<JournalEntry>>>;

/// Which extractor was selected and is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractorKind {
    ClaudeCode,
    Codex,
}

impl ExtractorKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex CLI / Desktop",
        }
    }
}

/// All active extractors, merged at extraction time.
pub struct ActiveExtractor {
    sources: Vec<(ExtractorKind, ExtractFn)>,
}

impl ActiveExtractor {
    /// Extract entries from all available sources and merge by timestamp.
    ///
    /// Returns a list of which sources contributed entries (for diagnostics)
    /// alongside the merged, redacted entries.
    pub fn extract_all(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<(Vec<(ExtractorKind, usize)>, Vec<JournalEntry>)> {
        let mut all_entries: Vec<JournalEntry> = Vec::new();
        let mut contributing: Vec<(ExtractorKind, usize)> = Vec::new();

        for (kind, extractor) in &self.sources {
            let entries = extractor(since, until)?;
            if !entries.is_empty() {
                contributing.push((*kind, entries.len()));
                all_entries.extend(entries);
            }
        }

        all_entries.sort_by_key(|e| e.timestamp);

        Ok((contributing, redact_entries(all_entries)))
    }

    /// Primary source kind — `None` if no supported tool was detected.
    pub fn primary_kind(&self) -> Option<ExtractorKind> {
        self.sources.first().map(|(k, _)| *k)
    }
}

/// Apply redaction to the prompt field of every entry.
fn redact_entries(entries: Vec<JournalEntry>) -> Vec<JournalEntry> {
    entries
        .into_iter()
        .map(|mut e| {
            let (redacted, _) = redact(&e.prompt);
            e.prompt = redacted;
            e
        })
        .collect()
}

/// Detect and return all available extractors for `project_root`.
pub fn detect(project_root: &Path, _project_id: &str) -> ActiveExtractor {
    let mut sources: Vec<(ExtractorKind, ExtractFn)> = Vec::new();

    if ClaudeCodeExtractor::is_available(project_root) {
        if let Some(log_dir) = ClaudeCodeExtractor::log_dir_for(project_root) {
            let ex = ClaudeCodeExtractor::new(log_dir);
            sources.push((
                ExtractorKind::ClaudeCode,
                Box::new(move |since, until| ex.extract(since, until)),
            ));
        }
    }

    if CodexExtractor::is_available(project_root) {
        if let Some(sessions_dir) = CodexExtractor::default_sessions_dir() {
            let ex = CodexExtractor::new(sessions_dir);
            sources.push((
                ExtractorKind::Codex,
                Box::new(move |since, until| ex.extract(since, until)),
            ));
        }
    }

    ActiveExtractor { sources }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn sample_entry(prompt: &str) -> JournalEntry {
        let mut e = JournalEntry::new(
            "feature/test".to_string(),
            "abc123".to_string(),
            prompt.to_string(),
            vec!["src/lib.rs".to_string()],
            vec!["Edit".to_string()],
            "done".to_string(),
            "claude-code".to_string(),
            None,
        );
        e.timestamp = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        e
    }

    fn window() -> (DateTime<Utc>, DateTime<Utc>) {
        let since = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        (since, since + Duration::hours(4))
    }

    #[test]
    fn extract_all_merges_multiple_sources() {
        let (since, until) = window();
        let ex = ActiveExtractor {
            sources: vec![
                (ExtractorKind::ClaudeCode, Box::new(|_, _| Ok(vec![sample_entry("from claude")]))),
                (ExtractorKind::Codex, Box::new(|_, _| Ok(vec![sample_entry("from codex")]))),
            ],
        };

        let (contributing, entries) = ex.extract_all(since, until).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(contributing.len(), 2);
        assert_eq!(contributing[0].0, ExtractorKind::ClaudeCode);
        assert_eq!(contributing[1].0, ExtractorKind::Codex);
    }

    #[test]
    fn extract_all_returns_empty_when_nothing_found() {
        let (since, until) = window();
        let ex = ActiveExtractor {
            sources: vec![
                (ExtractorKind::Codex, Box::new(|_, _| Ok(Vec::new()))),
            ],
        };

        let (contributing, entries) = ex.extract_all(since, until).unwrap();
        assert!(entries.is_empty());
        assert!(contributing.is_empty());
    }

    #[test]
    fn primary_kind_returns_none_when_no_sources() {
        let ex = ActiveExtractor { sources: vec![] };
        assert_eq!(ex.primary_kind(), None);
    }

    #[test]
    fn primary_kind_returns_first_source() {
        let ex = ActiveExtractor {
            sources: vec![
                (ExtractorKind::ClaudeCode, Box::new(|_, _| Ok(vec![]))),
            ],
        };
        assert_eq!(ex.primary_kind(), Some(ExtractorKind::ClaudeCode));
    }
}
