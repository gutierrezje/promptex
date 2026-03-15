//! Shared interface for tool-specific prompt extractors.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use super::ExtractorOutput;

/// A source that can produce prompt entries from an AI tool's session logs.
pub trait PromptExtractor {
    /// Returns true if this tool's logs exist and are readable for `project_root`.
    fn is_available(project_root: &Path) -> bool
    where
        Self: Sized;

    /// Extract prompt entries and non-fatal warnings within `[since, until]`.
    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<ExtractorOutput>;
}
