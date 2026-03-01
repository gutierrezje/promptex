//! Extractor detection and dispatch.
//!
//! `detect()` inspects the current environment and returns the best available
//! extractor. Priority: Claude Code → Codex → manual fallback.
//! If a supported extractor is selected but yields no entries for the
//! requested window, we automatically try manual journal entries before
//! returning an empty result.
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
pub mod manual;
pub mod opencode; // kept for future rewrite — not wired into detect()
pub mod traits;

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use crate::journal::JournalEntry;
use claude_code::ClaudeCodeExtractor;
use codex::CodexExtractor;
use manual::ManualExtractor;
use traits::PromptExtractor;

/// Which extractor was selected and is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractorKind {
    ClaudeCode,
    Codex,
    Manual,
}

impl ExtractorKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex CLI / Desktop",
            Self::Manual => "manual (pmtx record)",
        }
    }
}

/// The active extractor paired with its kind for display purposes.
pub struct ActiveExtractor {
    pub kind: ExtractorKind,
    extractor: Box<dyn Fn(DateTime<Utc>, DateTime<Utc>) -> Result<Vec<JournalEntry>>>,
    manual_fallback: Option<Box<dyn Fn(DateTime<Utc>, DateTime<Utc>) -> Result<Vec<JournalEntry>>>>,
}

impl ActiveExtractor {
    /// Extract entries and report which source provided the final result.
    ///
    /// When a non-manual extractor returns zero entries, we try manual journal
    /// fallback for the same window before returning.
    pub fn extract_with_source(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<(ExtractorKind, Vec<JournalEntry>)> {
        let entries = (self.extractor)(since, until)?;
        if !entries.is_empty() || self.kind == ExtractorKind::Manual {
            return Ok((self.kind, entries));
        }

        if let Some(fallback) = &self.manual_fallback {
            let manual_entries = fallback(since, until)?;
            if !manual_entries.is_empty() {
                return Ok((ExtractorKind::Manual, manual_entries));
            }
        }

        Ok((self.kind, entries))
    }
}

/// Detect and return the best extractor for `project_root`.
///
/// Falls back to the manual extractor if no tool logs are found, and also
/// when a selected supported extractor returns zero entries for the window.
pub fn detect(project_root: &Path, project_id: &str) -> ActiveExtractor {
    // 1. Claude Code
    if ClaudeCodeExtractor::is_available(project_root) {
        if let Some(log_dir) = ClaudeCodeExtractor::log_dir_for(project_root) {
            let ex = ClaudeCodeExtractor::new(log_dir);
            let pid = project_id.to_string();
            return ActiveExtractor {
                kind: ExtractorKind::ClaudeCode,
                extractor: Box::new(move |since, until| ex.extract(since, until)),
                manual_fallback: Some(Box::new(move |since, until| {
                    ManualExtractor::new(pid.clone()).extract(since, until)
                })),
            };
        }
    }

    // 2. Codex CLI / Desktop app (same log format, same ~/.codex/sessions/ path)
    if CodexExtractor::is_available(project_root) {
        if let Some(sessions_dir) = CodexExtractor::default_sessions_dir() {
            let ex = CodexExtractor::new(sessions_dir);
            let pid = project_id.to_string();
            return ActiveExtractor {
                kind: ExtractorKind::Codex,
                extractor: Box::new(move |since, until| ex.extract(since, until)),
                manual_fallback: Some(Box::new(move |since, until| {
                    ManualExtractor::new(pid.clone()).extract(since, until)
                })),
            };
        }
    }

    // 3. Manual fallback (pmtx record journal)
    let pid = project_id.to_string();
    ActiveExtractor {
        kind: ExtractorKind::Manual,
        extractor: Box::new(move |since, until| {
            ManualExtractor::new(pid.clone()).extract(since, until)
        }),
        manual_fallback: None,
    }
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
            "manual".to_string(),
            None,
        );
        e.timestamp = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        e
    }

    #[test]
    fn extract_with_source_uses_primary_when_non_empty() {
        let ex = ActiveExtractor {
            kind: ExtractorKind::Codex,
            extractor: Box::new(|_, _| Ok(vec![sample_entry("from primary")])),
            manual_fallback: Some(Box::new(|_, _| Ok(vec![sample_entry("from manual")]))),
        };

        let since = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let until = since + Duration::hours(4);
        let (kind, entries) = ex.extract_with_source(since, until).unwrap();

        assert_eq!(kind, ExtractorKind::Codex);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prompt, "from primary");
    }

    #[test]
    fn extract_with_source_falls_back_to_manual_when_primary_empty() {
        let ex = ActiveExtractor {
            kind: ExtractorKind::Codex,
            extractor: Box::new(|_, _| Ok(Vec::new())),
            manual_fallback: Some(Box::new(|_, _| Ok(vec![sample_entry("from manual")]))),
        };

        let since = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let until = since + Duration::hours(4);
        let (kind, entries) = ex.extract_with_source(since, until).unwrap();

        assert_eq!(kind, ExtractorKind::Manual);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prompt, "from manual");
    }

    #[test]
    fn extract_with_source_keeps_primary_kind_when_both_empty() {
        let ex = ActiveExtractor {
            kind: ExtractorKind::Codex,
            extractor: Box::new(|_, _| Ok(Vec::new())),
            manual_fallback: Some(Box::new(|_, _| Ok(Vec::new()))),
        };

        let since = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let until = since + Duration::hours(4);
        let (kind, entries) = ex.extract_with_source(since, until).unwrap();

        assert_eq!(kind, ExtractorKind::Codex);
        assert!(entries.is_empty());
    }
}
