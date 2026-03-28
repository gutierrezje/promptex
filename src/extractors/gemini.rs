//! Extract prompts from Gemini CLI session transcripts.
//!
//! Gemini CLI stores project mappings in `~/.gemini/projects.json` and session
//! JSON files in `~/.gemini/tmp/{slug}/chats/`.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use super::ExtractorOutput;
use crate::prompt::PromptEntry;

/// Extracts Gemini CLI JSON sessions for a single project.
pub struct GeminiCliExtractor {
    /// Gemini CLI log directory for the current project.
    project_log_dir: PathBuf,
    /// Project root used to relativize absolute paths found in transcripts.
    project_root: PathBuf,
}

impl GeminiCliExtractor {
    pub fn new(project_log_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            project_log_dir,
            project_root,
        }
    }

    /// Resolve the Gemini CLI log directory for `project_root`.
    pub fn log_dir_for(project_root: &Path) -> Option<PathBuf> {
        let home = home_dir()?;
        let gemini_dir = home.join(".gemini");
        let projects_json = gemini_dir.join("projects.json");

        if !projects_json.exists() {
            return None;
        }

        let file = File::open(projects_json).ok()?;
        let reader = BufReader::new(file);
        let data: Value = serde_json::from_reader(reader).ok()?;

        let projects = data.get("projects")?.as_object()?;
        let root_str = project_root.to_string_lossy().to_string();

        let slug = projects.get(&root_str)?.as_str()?;

        let chats_dir = gemini_dir.join("tmp").join(slug).join("chats");
        if chats_dir.exists() {
            Some(chats_dir)
        } else {
            None
        }
    }
}

impl PromptExtractor for GeminiCliExtractor {
    fn is_available(project_root: &Path) -> bool {
        Self::log_dir_for(project_root).is_some()
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<ExtractorOutput> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        let mut session_files: Vec<PathBuf> = fs::read_dir(&self.project_log_dir)
            .context("Failed to read Gemini CLI project log directory")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
            .collect();

        session_files.sort();

        for session_file in session_files {
            match extract_from_session(&session_file, since, until, &self.project_root) {
                Ok(mut session_out) => {
                    entries.append(&mut session_out.entries);
                    warnings.append(&mut session_out.warnings);
                }
                Err(err) => {
                    warnings.push(format!(
                        "failed to extract session from {}: {err}",
                        session_file.display()
                    ));
                }
            }
        }

        entries.sort_by_key(|e| e.timestamp);
        Ok(ExtractorOutput { entries, warnings })
    }
}

struct SessionExtractOutput {
    entries: Vec<PromptEntry>,
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SessionFile {
    messages: Option<Vec<RawMessage>>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    #[serde(rename = "type")]
    msg_type: String,
    timestamp: Option<DateTime<Utc>>,
    content: Option<Value>,
    #[serde(rename = "toolCalls")]
    tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
struct RawToolCall {
    name: String,
    args: Option<Value>,
}

fn extract_from_session(
    path: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    project_root: &Path,
) -> Result<SessionExtractOutput> {
    let file = File::open(path).context("Failed to open session file")?;
    let reader = BufReader::new(file);

    let mut warnings = Vec::new();
    let session: SessionFile = match serde_json::from_reader(reader) {
        Ok(s) => s,
        Err(e) => {
            warnings.push(format!("skipped invalid JSON file {}: {e}", path.display()));
            return Ok(SessionExtractOutput {
                entries: vec![],
                warnings,
            });
        }
    };

    let raw_messages = session.messages.unwrap_or_default();
    let mut entries = Vec::new();
    let mut i = 0;

    while i < raw_messages.len() {
        let msg = &raw_messages[i];

        if msg.msg_type == "user" {
            if let Some(ts) = msg.timestamp {
                if ts >= since && ts <= until {
                    if let Some(prompt_text) = extract_user_text(msg) {
                        let branch = "unknown".to_string(); // Gemini CLI sessions don't store branch

                        let (mut tool_calls, files_touched) =
                            collect_agent_context(&raw_messages, i + 1);
                        if detect_skill_usage(&files_touched) {
                            push_unique(&mut tool_calls, "Skill");
                        }

                        let mut normalized_files_touched = Vec::new();
                        for file in files_touched {
                            if let Some(normalized) =
                                normalize_files_touched_path(&file, project_root)
                            {
                                if !normalized_files_touched.contains(&normalized) {
                                    normalized_files_touched.push(normalized);
                                }
                            }
                        }

                        let mut entry = PromptEntry::new(
                            branch,
                            String::new(),
                            prompt_text,
                            normalized_files_touched,
                            tool_calls,
                            "gemini-cli".to_string(),
                            None,
                        );
                        entry.assistant_context = extract_preceding_context(&raw_messages, i);
                        entry.timestamp = ts;
                        entries.push(entry);
                    }
                }
            }
        }
        i += 1;
    }

    Ok(SessionExtractOutput { entries, warnings })
}

fn extract_user_text(msg: &RawMessage) -> Option<String> {
    let content = msg.content.as_ref()?;
    if let Value::Array(parts) = content {
        let mut text_parts = Vec::new();
        for part in parts {
            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    text_parts.push(trimmed.to_string());
                }
            }
        }
        let text = text_parts.join("\n");
        if !text.is_empty() {
            Some(text)
        } else {
            None
        }
    } else {
        None
    }
}

fn collect_agent_context(messages: &[RawMessage], start: usize) -> (Vec<String>, Vec<String>) {
    let mut tool_calls = Vec::new();
    let mut files_touched = Vec::new();

    for msg in messages[start..].iter() {
        if msg.msg_type == "user" {
            break;
        }

        if msg.msg_type != "gemini" && msg.msg_type != "agent" {
            continue;
        }

        if let Some(calls) = &msg.tool_calls {
            for call in calls {
                if call.name == "run_shell_command" {
                    let cmd = call.args.as_ref().and_then(extract_command_from_tool_input);
                    let inferred = cmd
                        .as_deref()
                        .map(infer_command_categories)
                        .unwrap_or_default();
                    if inferred.is_empty() {
                        push_unique(&mut tool_calls, "Bash");
                    } else {
                        for category in inferred {
                            push_unique(&mut tool_calls, &category);
                        }
                    }
                } else if call.name == "activate_skill" {
                    push_unique(&mut tool_calls, "Skill");
                    if let Some(skill_name) = call
                        .args
                        .as_ref()
                        .and_then(|a| a.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        files_touched.push(format!("skills/{}", skill_name));
                    }
                } else {
                    let tool_name = normalize_tool_name(&call.name);
                    push_unique(&mut tool_calls, &tool_name);
                }

                if let Some(file) = extract_file_from_tool(&call.name, call.args.as_ref()) {
                    if !files_touched.contains(&file) {
                        files_touched.push(file);
                    }
                }
            }
        }
    }

    (tool_calls, files_touched)
}

fn extract_preceding_context(messages: &[RawMessage], before_idx: usize) -> Option<String> {
    for msg in messages[..before_idx].iter().rev() {
        if msg.msg_type == "user" {
            break;
        }
        if msg.msg_type == "gemini" || msg.msg_type == "agent" {
            if let Some(Value::String(text)) = &msg.content {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let count = trimmed.chars().count();
                    let tail: String = if count <= 300 {
                        trimmed.to_string()
                    } else {
                        trimmed.chars().skip(count - 300).collect()
                    };
                    return Some(tail);
                }
            }
        }
    }
    None
}

fn normalize_tool_name(raw: &str) -> String {
    match raw {
        "replace" | "write_file" => "Write",
        "run_shell_command" => "Bash",
        "read_file" => "Read",
        "list_directory" | "grep_search" | "glob" | "codebase_investigator" => "Explore",
        _ => raw,
    }
    .to_string()
}

fn push_unique(vec: &mut Vec<String>, item: &str) {
    let s = item.to_string();
    if !vec.contains(&s) {
        vec.push(s);
    }
}

fn detect_skill_usage(paths: &[String]) -> bool {
    paths.iter().any(|path| {
        path.starts_with("skills/")
            || path.contains("/skills/")
            || path.contains("/.agents/skills/")
            || path.contains("/.codex/skills/")
    })
}

fn extract_command_from_tool_input(input: &Value) -> Option<String> {
    input
        .get("command")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn infer_command_categories(cmd: &str) -> Vec<String> {
    let mut categories = Vec::new();
    let sanitized = cmd.replace("&&", ";").replace("||", ";");
    for segment in sanitized.split([';', '|', '\n']) {
        let Some(head) = command_head(segment) else {
            continue;
        };
        let head = head.to_ascii_lowercase();
        let category = match head.as_str() {
            "cat" | "sed" | "head" | "tail" | "less" | "more" | "bat" | "jq" => Some("Read"),
            "rg" | "grep" | "ripgrep" | "ls" | "find" | "fd" | "tree" | "pwd" => Some("Explore"),
            "touch" | "tee" | "printf" | "echo" | "cp" | "mv" | "rm" | "mkdir" => Some("Write"),
            _ => None,
        };
        if let Some(cat) = category {
            push_unique(&mut categories, cat);
        }
    }
    categories
}

fn command_head(segment: &str) -> Option<&str> {
    let iter = segment.split_whitespace();
    for word in iter {
        if word == "sudo" || word == "env" {
            continue;
        }
        if word.contains('=') && !word.contains('/') {
            continue;
        }
        return Some(word);
    }
    None
}

fn extract_file_from_tool(tool_name: &str, input: Option<&Value>) -> Option<String> {
    let input = input?;
    let path = match tool_name {
        "replace" | "write_file" | "read_file" => input.get("file_path").and_then(|v| v.as_str()),
        "list_directory" | "glob" => input.get("dir_path").and_then(|v| v.as_str()),
        "run_shell_command" => None,
        _ => input
            .get("file_path")
            .or_else(|| input.get("dir_path"))
            .and_then(|v| v.as_str()),
    }?;

    Some(path.to_string())
}

fn normalize_files_touched_path(path: &str, project_root: &Path) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "~" || trimmed.starts_with("~/") {
        return None;
    }

    let p = Path::new(trimmed);
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }

    if p.is_absolute() {
        return p
            .strip_prefix(project_root)
            .ok()
            .map(|rel| rel.to_string_lossy().into_owned())
            .filter(|rel| !rel.is_empty());
    }

    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_tool_names() {
        assert_eq!(normalize_tool_name("run_shell_command"), "Bash");
        assert_eq!(normalize_tool_name("replace"), "Write");
        assert_eq!(normalize_tool_name("write_file"), "Write");
        assert_eq!(normalize_tool_name("read_file"), "Read");
        assert_eq!(normalize_tool_name("glob"), "Explore");
    }

    #[test]
    fn test_extract_user_text() {
        let msg = RawMessage {
            msg_type: "user".to_string(),
            timestamp: None,
            content: Some(serde_json::json!([{"text": "hello"}])),
            tool_calls: None,
        };
        assert_eq!(extract_user_text(&msg), Some("hello".to_string()));
    }

    #[test]
    fn test_collect_agent_context() {
        let messages = vec![
            RawMessage {
                msg_type: "gemini".to_string(),
                timestamp: None,
                content: None,
                tool_calls: Some(vec![
                    RawToolCall {
                        name: "run_shell_command".to_string(),
                        args: Some(serde_json::json!({"command": "ls"})),
                    },
                    RawToolCall {
                        name: "read_file".to_string(),
                        args: Some(serde_json::json!({"file_path": "src/main.rs"})),
                    },
                ]),
            },
            RawMessage {
                msg_type: "user".to_string(),
                timestamp: None,
                content: None,
                tool_calls: None,
            },
        ];

        let (tools, files) = collect_agent_context(&messages, 0);
        assert!(tools.contains(&"Explore".to_string())); // ls
        assert!(tools.contains(&"Read".to_string())); // read_file
        assert_eq!(files, vec!["src/main.rs"]);
    }
}
