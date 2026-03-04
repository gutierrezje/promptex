//! Extractor for Claude Code session transcripts.
//!
//! Claude Code writes append-only JSONL session files to:
//!   ~/.claude/projects/{slug}/{sessionId}.jsonl
//!
//! Each line is an independent JSON object. We look for `user` messages
//! that contain `text` content (the actual prompt), then collect the
//! tool calls and touched files from the following `assistant` message.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use crate::prompt::PromptEntry;

pub struct ClaudeCodeExtractor {
    /// The ~/.claude/projects/{slug}/ directory for this project.
    project_log_dir: PathBuf,
}

impl ClaudeCodeExtractor {
    pub fn new(project_log_dir: PathBuf) -> Self {
        Self { project_log_dir }
    }

    /// Resolve the Claude Code log directory for `project_root`.
    ///
    /// Claude Code slugifies the absolute project path for the directory name.
    pub fn log_dir_for(project_root: &Path) -> Option<PathBuf> {
        let home = home_dir()?;
        let claude_projects = home.join(".claude").join("projects");

        // Claude Code slugifies the path: replaces '/' with '-'
        // The leading '-' is intentional and part of the slug (e.g. -Users-alice-myproject)
        let slug = project_root.to_string_lossy().replace('/', "-");

        let candidate = claude_projects.join(&slug);
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    }
}

impl PromptExtractor for ClaudeCodeExtractor {
    fn is_available(project_root: &Path) -> bool {
        Self::log_dir_for(project_root).is_some()
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<PromptEntry>> {
        let mut entries = Vec::new();

        // Collect all *.jsonl files in the project log dir
        let mut session_files: Vec<PathBuf> = fs::read_dir(&self.project_log_dir)
            .context("Failed to read Claude Code project log directory")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();

        session_files.sort(); // chronological by filename (sessionId is time-based)

        for session_file in session_files {
            let mut file_entries =
                extract_from_session(&session_file, since, until).unwrap_or_default();
            entries.append(&mut file_entries);
        }

        entries.sort_by_key(|e| e.timestamp);
        Ok(entries)
    }
}

/// Prompts shorter than this are eligible for `assistant_context` capture.
const SHORT_PROMPT_WORD_THRESHOLD: usize = 8;
/// Max characters to store from the preceding assistant turn.
const MAX_CONTEXT_CHARS: usize = 300;

// ── Session parsing ───────────────────────────────────────────────────────────

/// A raw message line from a Claude Code JSONL session file.
#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(rename = "type")]
    msg_type: String,
    message: Option<MessageBody>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct MessageBody {
    role: Option<String>,
    content: Option<Value>, // string or array
}

fn extract_from_session(
    path: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Vec<PromptEntry>> {
    let file = File::open(path).context("Failed to open session file")?;
    let reader = BufReader::new(file);

    let mut raw_messages: Vec<RawMessage> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<RawMessage>(&line) {
            raw_messages.push(msg);
        }
    }

    // Walk messages: pair each user prompt with the assistant response that follows
    let mut entries = Vec::new();
    let mut i = 0;

    while i < raw_messages.len() {
        let msg = &raw_messages[i];

        if msg.msg_type == "user" {
            if let Some(ts) = msg.timestamp {
                if ts >= since && ts <= until {
                    if let Some(prompt_text) = extract_user_text(msg) {
                        let branch = msg
                            .git_branch
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());

                        // Collect tool calls and files from subsequent assistant messages
                        let (tool_calls, files_touched) =
                            collect_assistant_context(&raw_messages, i + 1);

                        let mut entry = PromptEntry::new(
                            branch,
                            String::new(), // commit hash not in logs; filled by correlation
                            prompt_text.clone(),
                            files_touched,
                            tool_calls,
                            String::new(), // outcome inferred during curation
                            "claude-code".to_string(),
                            None,
                        );
                        // Capture preceding assistant question for short replies
                        if prompt_text.split_whitespace().count() < SHORT_PROMPT_WORD_THRESHOLD {
                            entry.assistant_context = extract_preceding_question(&raw_messages, i);
                        }
                        // Override timestamp with the actual log timestamp
                        entries.push(with_timestamp(entry, ts));
                    }
                }
            }
        }

        i += 1;
    }

    Ok(entries)
}

/// Extract plain text from a user message (content can be string or array).
fn extract_user_text(msg: &RawMessage) -> Option<String> {
    let body = msg.message.as_ref()?;
    if body.role.as_deref() != Some("user") {
        return None;
    }
    let content = body.content.as_ref()?;

    match content {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Array(parts) => {
            // Concatenate all text-type content blocks
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
        _ => None,
    }
}

/// Extract plain text blocks from an assistant message.
fn extract_assistant_text(msg: &RawMessage) -> Option<String> {
    let content = msg.message.as_ref()?.content.as_ref()?;
    if let Value::Array(parts) = content {
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
        if !text.is_empty() {
            Some(text)
        } else {
            None
        }
    } else {
        None
    }
}

/// Return the tail of the most recent assistant turn before `before_idx`.
///
/// Walking backward stops at the previous user message so we don't pull in
/// context from an unrelated earlier exchange.
fn extract_preceding_question(messages: &[RawMessage], before_idx: usize) -> Option<String> {
    for msg in messages[..before_idx].iter().rev() {
        if msg.msg_type == "user" {
            break;
        }
        if msg.msg_type == "assistant" {
            if let Some(text) = extract_assistant_text(msg) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    // Take the tail — proposals and questions tend to be at the end.
                    let count = trimmed.chars().count();
                    let tail: String = if count <= MAX_CONTEXT_CHARS {
                        trimmed.to_string()
                    } else {
                        trimmed.chars().skip(count - MAX_CONTEXT_CHARS).collect()
                    };
                    return Some(tail);
                }
            }
        }
    }
    None
}

/// Walk forward from `start` collecting tool names and file paths until the
/// next user message (i.e., within a single assistant response turn).
fn collect_assistant_context(messages: &[RawMessage], start: usize) -> (Vec<String>, Vec<String>) {
    let mut tool_calls = Vec::new();
    let mut files_touched = Vec::new();

    for msg in messages[start..].iter() {
        // Stop at the next user turn
        if msg.msg_type == "user" {
            break;
        }

        if msg.msg_type != "assistant" {
            continue;
        }

        let content = match msg.message.as_ref().and_then(|b| b.content.as_ref()) {
            Some(c) => c,
            None => continue,
        };

        if let Value::Array(parts) = content {
            for part in parts {
                let part_type = part.get("type").and_then(|v| v.as_str()).unwrap_or("");

                if part_type == "tool_use" {
                    if let Some(name) = part.get("name").and_then(|v| v.as_str()) {
                        let tool_name = normalize_tool_name(name);
                        if !tool_calls.contains(&tool_name) {
                            tool_calls.push(tool_name);
                        }

                        // Extract file path from tool input if present
                        if let Some(file) = extract_file_from_tool(name, part.get("input")) {
                            if !files_touched.contains(&file) {
                                files_touched.push(file);
                            }
                        }
                    }
                }
            }
        }
    }

    (tool_calls, files_touched)
}

fn normalize_tool_name(raw: &str) -> String {
    // Claude Code tool names: "str_replace_based_edit_tool" → "Edit", "bash" → "Bash", etc.
    match raw {
        "str_replace_based_edit_tool" | "write_file" => "Edit",
        "bash" => "Bash",
        "read_file" => "Read",
        "list_directory" => "LS",
        "search_files" => "Grep",
        "glob_files" => "Glob",
        _ => raw,
    }
    .to_string()
}

fn extract_file_from_tool(tool_name: &str, input: Option<&Value>) -> Option<String> {
    let input = input?;
    let path = match tool_name {
        "str_replace_based_edit_tool" | "write_file" | "read_file" => {
            input.get("path").and_then(|v| v.as_str())
        }
        "bash" => None, // file paths in bash args are too noisy to extract reliably
        _ => input.get("path").and_then(|v| v.as_str()),
    }?;

    Some(path.to_string())
}

/// Return a copy of `entry` with its timestamp overridden.
fn with_timestamp(mut entry: PromptEntry, ts: DateTime<Utc>) -> PromptEntry {
    entry.timestamp = ts;
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_false_for_nonexistent_project() {
        let fake = Path::new("/tmp/nonexistent-project-xyz");
        assert!(!ClaudeCodeExtractor::is_available(fake));
    }

    #[test]
    fn test_normalize_tool_names() {
        assert_eq!(normalize_tool_name("bash"), "Bash");
        assert_eq!(normalize_tool_name("str_replace_based_edit_tool"), "Edit");
        assert_eq!(normalize_tool_name("read_file"), "Read");
    }
}
