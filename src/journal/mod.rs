//! Journal module for prompt logging and retrieval
//!
//! The journal is an append-only JSONL file stored in ~/.promptex/projects/<id>/journal.jsonl
//! Each line is a JournalEntry containing one prompt and its context.

pub mod entry;
pub mod reader;
pub mod writer;

// Re-export commonly used types
pub use entry::JournalEntry;
pub use reader::{count_entries, load_journal, load_journal_for_branch};
pub use writer::{append_entries, append_entry};
