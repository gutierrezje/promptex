//! Extractor detection and dispatch.
//!
//! `detect()` inspects the current environment and returns the best available
//! extractor. Priority: Claude Code → OpenCode → Codex → manual fallback.

pub mod claude_code;
pub mod codex;
pub mod manual;
pub mod opencode;
pub mod traits;

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use crate::journal::JournalEntry;
use crate::project_id;

use claude_code::ClaudeCodeExtractor;
use codex::CodexExtractor;
use manual::ManualExtractor;
use opencode::OpenCodeExtractor;
use traits::PromptExtractor;

/// Which extractor was selected and is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractorKind {
    ClaudeCode,
    OpenCode,
    Codex,
    Manual,
}

impl ExtractorKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex CLI",
            Self::Manual => "manual (pmtx record)",
        }
    }
}

/// The active extractor paired with its kind for display purposes.
pub struct ActiveExtractor {
    pub kind: ExtractorKind,
    extractor: Box<dyn Fn(DateTime<Utc>, DateTime<Utc>) -> Result<Vec<JournalEntry>>>,
}

impl ActiveExtractor {
    pub fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>> {
        (self.extractor)(since, until)
    }
}

/// Detect and return the best extractor for `project_root`.
///
/// Falls back to the manual extractor if no tool logs are found.
pub fn detect(project_root: &Path, project_id: &str) -> ActiveExtractor {
    // 1. Claude Code
    if ClaudeCodeExtractor::is_available(project_root) {
        if let Some(log_dir) = ClaudeCodeExtractor::log_dir_for(project_root) {
            let ex = ClaudeCodeExtractor::new(log_dir);
            return ActiveExtractor {
                kind: ExtractorKind::ClaudeCode,
                extractor: Box::new(move |since, until| ex.extract(since, until)),
            };
        }
    }

    // 2. OpenCode
    if OpenCodeExtractor::is_available(project_root) {
        if let Some(msg_dir) = OpenCodeExtractor::default_message_dir() {
            let ex = OpenCodeExtractor::new(msg_dir);
            return ActiveExtractor {
                kind: ExtractorKind::OpenCode,
                extractor: Box::new(move |since, until| ex.extract(since, until)),
            };
        }
    }

    // 3. Codex CLI
    if CodexExtractor::is_available(project_root) {
        if let Some(sessions_dir) = CodexExtractor::default_sessions_dir() {
            let ex = CodexExtractor::new(sessions_dir);
            return ActiveExtractor {
                kind: ExtractorKind::Codex,
                extractor: Box::new(move |since, until| ex.extract(since, until)),
            };
        }
    }

    // 4. Manual fallback
    let pid = project_id.to_string();
    ActiveExtractor {
        kind: ExtractorKind::Manual,
        extractor: Box::new(move |since, until| {
            ManualExtractor::new(pid.clone()).extract(since, until)
        }),
    }
}
