//! Shared interface for tool-specific prompt extractors.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use super::ExtractorOutput;

/// A source that can produce prompt entries from an AI tool's session logs.
///
/// The trait is intentionally flexible because log formats vary widely
/// (per-project folders, global sessions, SQLite storage, etc.). While
/// extractor implementations vary internally, they are expected to emit
/// `PromptEntry` values that meet shared output contracts:
/// - Prompts are non-empty.
/// - Timestamps are within the requested window.
/// - Tool names are normalized to canonical labels (Read, Write, Bash, Explore, Skill).
/// - Touched files are repo-relative paths (no absolute paths or parent traversal).
pub trait PromptExtractor {
    /// Returns true if this tool's logs exist and are readable for `project_root`.
    fn is_available(project_root: &Path) -> bool
    where
        Self: Sized;

    /// Extract prompt entries and non-fatal warnings within `[since, until]`.
    ///
    /// Callers will merge and sort results, so ordering is not required but
    /// should be deterministic if possible.
    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<ExtractorOutput>;
}
