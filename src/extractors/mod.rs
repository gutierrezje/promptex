//! Detect and run prompt extractors.
//!
//! More than one extractor may be active in a single workspace, so detection
//! is intentionally additive. The merged output is sorted by timestamp before
//! it is returned to the caller.
//!
//! ## Extractor support status
//! | Tool           | Status  | Notes                                           |
//! |----------------|---------|------------------------------------------------ |
//! | Claude Code    | ✅ Active | JSONL sessions at `~/.claude/projects/`        |
//! | Codex CLI/App  | 🚧 WIP    | JSONL sessions at `~/.codex/sessions/`         |
//! | OpenCode       | ⏳ TODO  | Migrated to SQLite (v1.2+); needs rewrite       |
//! | Cursor         | ⏳ TODO  | Log format TBD                                  |
//! | GitHub Copilot | ⏳ TODO  | Log format TBD                                  |

pub mod claude_code;
pub mod codex;
pub mod opencode; // kept for future rewrite — not wired into detect()
pub mod traits;

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::path::Path;

use crate::curation::redact::redact;
use crate::prompt::PromptEntry;
use claude_code::ClaudeCodeExtractor;
use codex::CodexExtractor;
use traits::PromptExtractor;

type ExtractFn = Box<dyn Fn(DateTime<Utc>, DateTime<Utc>) -> Result<ExtractorOutput>>;
type ExtractResult = Result<(
    Vec<(ExtractorKind, usize)>,
    Vec<PromptEntry>,
    ExtractionDiagnostics,
)>;

/// Extracted entries and non-fatal warnings from a single source run.
#[derive(Debug, Default)]
pub struct ExtractorOutput {
    pub entries: Vec<PromptEntry>,
    pub warnings: Vec<String>,
}

/// Non-fatal warning captured during extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionWarning {
    pub source: ExtractorKind,
    pub detail: String,
}

/// Diagnostics across all extractor runs.
#[derive(Debug, Default, Clone)]
pub struct ExtractionDiagnostics {
    pub warnings: Vec<ExtractionWarning>,
}

impl ExtractionDiagnostics {
    pub fn warning_count_by_source(&self) -> BTreeMap<ExtractorKind, usize> {
        let mut counts = BTreeMap::new();
        for warning in &self.warnings {
            *counts.entry(warning.source).or_insert(0) += 1;
        }
        counts
    }
}

/// Extractor source.
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

    pub fn readiness(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "supported",
            Self::Codex => "WIP",
        }
    }
}

impl Ord for ExtractorKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.label().cmp(other.label())
    }
}

impl PartialOrd for ExtractorKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Active extractors for the current workspace.
pub struct ActiveExtractor {
    sources: Vec<(ExtractorKind, ExtractFn)>,
}

impl ActiveExtractor {
    /// Extract, merge, and redact entries from every detected source.
    pub fn extract_all(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> ExtractResult {
        let mut all_entries: Vec<PromptEntry> = Vec::new();
        let mut contributing: Vec<(ExtractorKind, usize)> = Vec::new();
        let mut diagnostics = ExtractionDiagnostics::default();

        for (kind, extractor) in &self.sources {
            let output = extractor(since, until)?;

            if !output.entries.is_empty() {
                contributing.push((*kind, output.entries.len()));
                all_entries.extend(output.entries);
            }

            diagnostics
                .warnings
                .extend(output.warnings.into_iter().map(|detail| ExtractionWarning {
                    source: *kind,
                    detail,
                }));
        }

        if !all_entries.is_empty() {
            all_entries.sort_by_key(|e| e.timestamp);
        }

        Ok((contributing, redact_entries(all_entries), diagnostics))
    }

    /// The first detected source, if any.
    pub fn primary_kind(&self) -> Option<ExtractorKind> {
        self.sources.first().map(|(k, _)| *k)
    }
}

/// Apply prompt redaction to every extracted entry.
fn redact_entries(entries: Vec<PromptEntry>) -> Vec<PromptEntry> {
    entries
        .into_iter()
        .map(|mut e| {
            let (redacted, _) = redact(&e.prompt);
            e.prompt = redacted;
            if let Some(ctx) = e.assistant_context.take() {
                let (redacted_ctx, _) = redact(&ctx);
                e.assistant_context = Some(redacted_ctx);
            }
            e
        })
        .collect()
}

/// Detect every extractor that appears available for `project_root`.
pub fn detect(project_root: &Path, _project_id: &str) -> ActiveExtractor {
    let mut sources: Vec<(ExtractorKind, ExtractFn)> = Vec::new();

    if ClaudeCodeExtractor::is_available(project_root) {
        if let Some(log_dir) = ClaudeCodeExtractor::log_dir_for(project_root) {
            let ex = ClaudeCodeExtractor::new(log_dir, project_root.to_path_buf());
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

    fn sample_entry(prompt: &str) -> PromptEntry {
        let mut e = PromptEntry::new(
            "feature/test".to_string(),
            "abc123".to_string(),
            prompt.to_string(),
            vec!["src/lib.rs".to_string()],
            vec!["Edit".to_string()],
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
                (
                    ExtractorKind::ClaudeCode,
                    Box::new(|_, _| {
                        Ok(ExtractorOutput {
                            entries: vec![sample_entry("from claude")],
                            warnings: Vec::new(),
                        })
                    }),
                ),
                (
                    ExtractorKind::Codex,
                    Box::new(|_, _| {
                        Ok(ExtractorOutput {
                            entries: vec![sample_entry("from codex")],
                            warnings: Vec::new(),
                        })
                    }),
                ),
            ],
        };

        let (contributing, entries, diagnostics) = ex.extract_all(since, until).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(contributing.len(), 2);
        assert_eq!(contributing[0].0, ExtractorKind::ClaudeCode);
        assert_eq!(contributing[1].0, ExtractorKind::Codex);
        assert!(diagnostics.warnings.is_empty());
    }

    #[test]
    fn extract_all_returns_empty_when_nothing_found() {
        let (since, until) = window();
        let ex = ActiveExtractor {
            sources: vec![(
                ExtractorKind::Codex,
                Box::new(|_, _| {
                    Ok(ExtractorOutput {
                        entries: Vec::new(),
                        warnings: Vec::new(),
                    })
                }),
            )],
        };

        let (contributing, entries, diagnostics) = ex.extract_all(since, until).unwrap();
        assert!(entries.is_empty());
        assert!(contributing.is_empty());
        assert!(diagnostics.warnings.is_empty());
    }

    #[test]
    fn primary_kind_returns_none_when_no_sources() {
        let ex = ActiveExtractor { sources: vec![] };
        assert_eq!(ex.primary_kind(), None);
    }

    #[test]
    fn primary_kind_returns_first_source() {
        let ex = ActiveExtractor {
            sources: vec![(
                ExtractorKind::ClaudeCode,
                Box::new(|_, _| {
                    Ok(ExtractorOutput {
                        entries: vec![],
                        warnings: vec![],
                    })
                }),
            )],
        };
        assert_eq!(ex.primary_kind(), Some(ExtractorKind::ClaudeCode));
    }

    #[test]
    fn extract_all_redacts_prompt_and_assistant_context() {
        let (since, until) = window();
        let ex = ActiveExtractor {
            sources: vec![(
                ExtractorKind::ClaudeCode,
                Box::new(|_, _| {
                    let mut entry =
                        sample_entry("use this key: sk-abcdefghijklmnopqrstuvwxyz123456");
                    entry.assistant_context = Some(
                        "Authorization: Bearer abcdefghijklmnopqrstuvwxyz1234567890".to_string(),
                    );
                    Ok(ExtractorOutput {
                        entries: vec![entry],
                        warnings: vec![],
                    })
                }),
            )],
        };

        let (_, entries, _) = ex.extract_all(since, until).unwrap();
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert!(entry.prompt.contains("[REDACTED:api_key]"));

        let ctx = entry.assistant_context.as_ref().unwrap();
        assert!(ctx.contains("[REDACTED:bearer_token]"));
        assert!(!ctx.contains("abcdefghijklmnopqrstuvwxyz1234567890"));
    }

    #[test]
    fn extract_all_returns_non_fatal_warnings() {
        let (since, until) = window();
        let ex = ActiveExtractor {
            sources: vec![
                (
                    ExtractorKind::ClaudeCode,
                    Box::new(|_, _| {
                        Ok(ExtractorOutput {
                            entries: vec![sample_entry("from claude")],
                            warnings: vec!["bad json line in a.jsonl:12".to_string()],
                        })
                    }),
                ),
                (
                    ExtractorKind::Codex,
                    Box::new(|_, _| {
                        Ok(ExtractorOutput {
                            entries: vec![],
                            warnings: vec![
                                "failed to parse rollout file b.jsonl".to_string(),
                                "failed to parse rollout file c.jsonl".to_string(),
                            ],
                        })
                    }),
                ),
            ],
        };

        let (_, entries, diagnostics) = ex.extract_all(since, until).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(diagnostics.warnings.len(), 3);
        let counts = diagnostics.warning_count_by_source();
        assert_eq!(counts.get(&ExtractorKind::ClaudeCode), Some(&1));
        assert_eq!(counts.get(&ExtractorKind::Codex), Some(&2));
    }
}
