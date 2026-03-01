//! Extractor for OpenAI Codex CLI session logs.
//!
//! Codex stores session rollouts at:
//!   ~/.codex/sessions/YYYY/MM/DD/rollout-{timestamp}-{uuid}.jsonl
//!
//! Each line is a JSON event. User prompts appear as messages with
//! role "user", assistant responses as role "assistant" with tool calls.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use crate::journal::JournalEntry;

pub struct CodexExtractor {
    sessions_dir: PathBuf,
}

impl CodexExtractor {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    pub fn default_sessions_dir() -> Option<PathBuf> {
        // Respects CODEX_HOME env var override
        let base = if let Ok(home) = std::env::var("CODEX_HOME") {
            PathBuf::from(home)
        } else {
            home_dir()?.join(".codex")
        };
        let dir = base.join("sessions");
        if dir.exists() { Some(dir) } else { None }
    }
}

#[derive(Debug, Deserialize)]
struct CodexEvent {
    #[serde(rename = "type")]
    event_type: Option<String>,
    payload: Option<Value>,
}

impl PromptExtractor for CodexExtractor {
    fn is_available(_project_root: &Path) -> bool {
        Self::default_sessions_dir().is_some()
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>> {
        let mut entries = Vec::new();

        // Collect all rollout-*.jsonl files recursively under sessions/
        let session_files = collect_jsonl_files(&self.sessions_dir);

        for file in session_files {
            let mut file_entries =
                extract_from_rollout(&file, since, until).unwrap_or_default();
            entries.append(&mut file_entries);
        }

        entries.sort_by_key(|e| e.timestamp);
        Ok(entries)
    }
}

fn collect_jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return files };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_jsonl_files(&path));
        } else if path.extension().map_or(false, |ext| ext == "jsonl") {
            files.push(path);
        }
    }

    files.sort();
    files
}

fn extract_from_rollout(
    path: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Vec<JournalEntry>> {
    let file = File::open(path).context("Failed to open Codex session file")?;
    let reader = BufReader::new(file);

    let mut events: Vec<Value> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&line) {
            events.push(v);
        }
    }

    let mut entries = Vec::new();
    let mut i = 0;

    while i < events.len() {
        let event = &events[i];

        // Codex user messages: type = "message", payload.role = "user"
        if let Some(text) = extract_user_prompt(event) {
            if let Some(ts) = extract_timestamp(event) {
                if ts >= since && ts <= until {
                    let (tool_calls, files_touched) = collect_tools(&events, i + 1);

                    let entry = JournalEntry::new(
                        "unknown".to_string(), // Codex doesn't embed git branch
                        String::new(),
                        text,
                        files_touched,
                        tool_calls,
                        String::new(),
                        "codex".to_string(),
                        extract_model(event).unwrap_or_default().into(),
                    );
                    entries.push(with_timestamp(entry, ts));
                }
            }
        }

        i += 1;
    }

    Ok(entries)
}

fn extract_user_prompt(event: &Value) -> Option<String> {
    // Format: { type: "message", payload: { role: "user", content: "..." } }
    let payload = event.get("payload")?;
    if payload.get("role")?.as_str()? != "user" {
        return None;
    }
    let content = payload.get("content")?;
    match content {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        }
        Value::Array(parts) => {
            let text: String = parts
                .iter()
                .filter_map(|p| {
                    if p.get("type")?.as_str()? == "text" {
                        p.get("text")?.as_str().map(String::from)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            if text.is_empty() { None } else { Some(text) }
        }
        _ => None,
    }
}

fn extract_timestamp(event: &Value) -> Option<DateTime<Utc>> {
    event
        .get("timestamp")
        .or_else(|| event.get("payload").and_then(|p| p.get("timestamp")))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn extract_model(event: &Value) -> Option<String> {
    event
        .get("payload")
        .and_then(|p| p.get("model"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn collect_tools(events: &[Value], start: usize) -> (Vec<String>, Vec<String>) {
    let mut tool_calls = Vec::new();
    let mut files_touched = Vec::new();

    for event in &events[start..] {
        let payload = match event.get("payload") {
            Some(p) => p,
            None => continue,
        };

        // Stop at the next user message
        if payload.get("role").and_then(|v| v.as_str()) == Some("user") {
            break;
        }

        // Tool call events
        if let Some(tool_name) = event
            .get("type")
            .and_then(|t| if t == "tool_call" { Some(t) } else { None })
            .and_then(|_| payload.get("name"))
            .and_then(|v| v.as_str())
        {
            let t = tool_name.to_string();
            if !tool_calls.contains(&t) {
                tool_calls.push(t);
            }

            if let Some(file) = payload
                .get("arguments")
                .and_then(|a| a.get("path").or_else(|| a.get("file_path")))
                .and_then(|v| v.as_str())
            {
                let f = file.to_string();
                if !files_touched.contains(&f) {
                    files_touched.push(f);
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
    fn test_collect_jsonl_files_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let files = collect_jsonl_files(dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn test_is_available_checks_directory() {
        let _ = CodexExtractor::is_available(Path::new("/tmp"));
    }
}
