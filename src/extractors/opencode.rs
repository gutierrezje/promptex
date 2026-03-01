//! Extractor for OpenCode (sst/opencode) session logs.
//!
//! ⚠️  NOT WIRED INTO DETECTION — needs rewrite before use.
//!
//! OpenCode v1.2+ migrated from JSON files to SQLite:
#![allow(dead_code, clippy::unnecessary_map_or, clippy::needless_return)]
//!   Old (≤v1.1): JSON files at ~/.local/share/opencode/storage/message/
//!   New (v1.2+):  SQLite at ~/.local/share/opencode/opencode.db
//!
//! This extractor targets the old JSON format and will silently return zero
//! entries on any current install. A future rewrite should query the SQLite
//! database directly. The MessageV2 schema (from opencode source):
//!   - MessageTable: id, session_id, role, model, time, ...
//!   - PartTable:    message_id, type ("text" | "tool" | "reasoning"), ...
//!   - ToolPart:     { type: "tool", tool: { toolName, ... }, state, ... }

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use crate::journal::JournalEntry;

pub struct OpenCodeExtractor {
    message_dir: PathBuf,
}

impl OpenCodeExtractor {
    pub fn new(message_dir: PathBuf) -> Self {
        Self { message_dir }
    }

    pub fn default_message_dir() -> Option<PathBuf> {
        let dir = home_dir()?
            .join(".local")
            .join("share")
            .join("opencode")
            .join("storage")
            .join("message");
        if dir.exists() {
            Some(dir)
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenCodeMessage {
    role: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<DateTime<Utc>>,
    parts: Option<Vec<Value>>,
}

impl PromptExtractor for OpenCodeExtractor {
    fn is_available(_project_root: &Path) -> bool {
        // OpenCode storage is global (not per-project), so just check it exists
        Self::default_message_dir().is_some()
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>> {
        let mut entries = Vec::new();

        let mut files: Vec<PathBuf> = fs::read_dir(&self.message_dir)
            .context("Failed to read OpenCode message directory")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "json"))
            .collect();

        files.sort();

        // Collect user messages, then pair with the next assistant message's tools
        let mut messages: Vec<(DateTime<Utc>, OpenCodeMessage)> = Vec::new();

        for file in &files {
            let f = File::open(file)?;
            let msg: OpenCodeMessage =
                serde_json::from_reader(BufReader::new(f)).unwrap_or_else(|_| {
                    return OpenCodeMessage {
                        role: None,
                        created_at: None,
                        parts: None,
                    };
                });

            if let Some(ts) = msg.created_at {
                if ts >= since && ts <= until {
                    messages.push((ts, msg));
                }
            }
        }

        messages.sort_by_key(|(ts, _)| *ts);

        let mut i = 0;
        while i < messages.len() {
            let (ts, msg) = &messages[i];
            if msg.role.as_deref() == Some("user") {
                if let Some(text) = extract_text_from_parts(msg.parts.as_deref()) {
                    let (tool_calls, files_touched) = collect_next_assistant(&messages, i + 1);

                    let entry = JournalEntry::new(
                        "unknown".to_string(), // OpenCode doesn't embed git branch in messages
                        String::new(),
                        text,
                        files_touched,
                        tool_calls,
                        String::new(),
                        "opencode".to_string(),
                        None,
                    );
                    entries.push(with_timestamp(entry, *ts));
                }
            }
            i += 1;
        }

        Ok(entries)
    }
}

fn extract_text_from_parts(parts: Option<&[Value]>) -> Option<String> {
    let parts = parts?;
    let text: String = parts
        .iter()
        .filter_map(|p| {
            if p.get("type")?.as_str()? == "text" {
                p.get("text")?.as_str().map(str::trim).map(String::from)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn collect_next_assistant(
    messages: &[(DateTime<Utc>, OpenCodeMessage)],
    start: usize,
) -> (Vec<String>, Vec<String>) {
    let mut tool_calls = Vec::new();
    let mut files_touched = Vec::new();

    for (_, msg) in messages[start..].iter() {
        if msg.role.as_deref() == Some("user") {
            break;
        }
        if msg.role.as_deref() != Some("assistant") {
            continue;
        }

        if let Some(parts) = &msg.parts {
            for part in parts {
                let part_type = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if part_type == "tool-invocation" || part_type == "tool_call" {
                    if let Some(name) = part
                        .get("toolInvocation")
                        .or_else(|| part.get("tool"))
                        .and_then(|t| t.get("toolName").or_else(|| t.get("name")))
                        .and_then(|v| v.as_str())
                    {
                        let tool = name.to_string();
                        if !tool_calls.contains(&tool) {
                            tool_calls.push(tool.clone());
                        }

                        // Try to extract file path from tool args
                        if let Some(file) = part
                            .get("toolInvocation")
                            .or_else(|| part.get("tool"))
                            .and_then(|t| t.get("args"))
                            .and_then(|a| a.get("filePath").or_else(|| a.get("path")))
                            .and_then(|v| v.as_str())
                        {
                            let f = file.to_string();
                            if !files_touched.contains(&f) {
                                files_touched.push(f);
                            }
                        }
                    }
                }
            }
        }
    }

    (tool_calls, files_touched)
}

fn with_timestamp(mut entry: JournalEntry, ts: DateTime<Utc>) -> JournalEntry {
    entry.timestamp = ts;
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_checks_directory() {
        // Just verify it doesn't panic — result depends on machine state
        let _ = OpenCodeExtractor::is_available(Path::new("/tmp"));
    }
}
