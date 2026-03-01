//! Fallback extractor — reads from the pmtx journal written by `pmtx record`.
//!
//! Used when no supported AI tool logs are detected. Also serves as the
//! primary source when the user manually journals prompts for unsupported tools.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;

use super::traits::PromptExtractor;
use crate::journal::{self, JournalEntry};
use crate::project_id;

pub struct ManualExtractor {
    project_id: String,
}

impl ManualExtractor {
    pub fn new(project_id: String) -> Self {
        Self { project_id }
    }
}

impl PromptExtractor for ManualExtractor {
    fn is_available(_project_root: &Path) -> bool {
        // Manual extractor is always available as a fallback — even if the
        // journal is empty it will just return an empty vec.
        true
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>> {
        let all = journal::load_journal(&self.project_id)?;

        Ok(all
            .into_iter()
            .filter(|e| e.timestamp >= since && e.timestamp <= until)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::writer::append_entry;
    use crate::journal::JournalEntry;
    use chrono::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_manual_extractor_filters_by_time() {
        let dir = TempDir::new().unwrap();
        let _guard = crate::project_id::set_test_home(dir.path());

        let now = Utc::now();

        // Entry inside the window
        let mut inside = JournalEntry::new(
            "main".to_string(),
            "abc123".to_string(),
            "prompt inside window".to_string(),
            vec![],
            vec![],
            "done".to_string(),
            "claude-code".to_string(),
            None,
        );
        inside.timestamp = now;

        // Entry outside the window (2 hours ago)
        let mut outside = JournalEntry::new(
            "main".to_string(),
            "abc123".to_string(),
            "prompt outside window".to_string(),
            vec![],
            vec![],
            "done".to_string(),
            "claude-code".to_string(),
            None,
        );
        outside.timestamp = now - Duration::hours(2);

        append_entry("test-manual", &inside).unwrap();
        append_entry("test-manual", &outside).unwrap();

        let extractor = ManualExtractor::new("test-manual".to_string());
        let since = now - Duration::minutes(30);
        let until = now + Duration::minutes(1);

        let results = extractor.extract(since, until).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].prompt, "prompt inside window");
    }
}
