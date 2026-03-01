//! PromptExtractor trait — implemented by each tool-specific extractor.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use crate::journal::JournalEntry;

/// A source that can produce journal entries from an AI tool's session logs.
pub trait PromptExtractor {
    /// Returns true if this tool's logs exist and are readable for `project_root`.
    fn is_available(project_root: &Path) -> bool
    where
        Self: Sized;

    /// Extract prompt entries whose timestamps fall within `[since, until]`.
    ///
    /// Entries are returned in chronological order (oldest first).
    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>>;
}
