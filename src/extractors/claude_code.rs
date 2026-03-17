//! Extract prompts from Claude Code session transcripts.
//!
//! Claude Code stores per-project JSONL transcripts under
//! `~/.claude/projects/{slug}/`. This extractor keeps human-authored user turns,
//! drops known runtime-injected noise, and captures the assistant tool activity
//! that follows each prompt.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use super::ExtractorOutput;
use crate::prompt::PromptEntry;

pub struct ClaudeCodeExtractor {
    /// Claude Code log directory for the current project.
    project_log_dir: PathBuf,
    /// Project root used to relativize absolute paths found in transcripts.
    project_root: PathBuf,
}

impl ClaudeCodeExtractor {
    pub fn new(project_log_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            project_log_dir,
            project_root,
        }
    }

    /// Resolve the Claude Code log directory for `project_root`.
    pub fn log_dir_for(project_root: &Path) -> Option<PathBuf> {
        let home = home_dir()?;
        let claude_projects = home.join(".claude").join("projects");

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

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<ExtractorOutput> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();

        let mut session_files: Vec<PathBuf> = fs::read_dir(&self.project_log_dir)
            .context("Failed to read Claude Code project log directory")?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();

        session_files.sort(); // chronological by filename (sessionId is time-based)

        for session_file in session_files {
            match extract_from_session(&session_file, since, until, &self.project_root) {
                Ok(mut session_out) => {
                    entries.append(&mut session_out.entries);
                    warnings.append(&mut session_out.warnings);
                }
                Err(err) => {
                    warnings.push(format!(
                        "{}: failed to extract session ({err})",
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

/// Max characters to store from the preceding assistant turn.
const MAX_CONTEXT_CHARS: usize = 300;

/// XML-tag prefixes that identify system-injected user turns, not real prompts.
///
/// Claude Code injects these into the conversation as `user` role messages, but
/// they are never authored by the human and should not appear in prompt history.
const JUNK_PREFIXES: &[&str] = &[
    "<command-name>",
    "<local-command-stdout>",
    "<local-command-caveat>",
    "<task-notification>",
    "<system-reminder>",
    "<command-message>",
];

/// Strip any leading junk XML blocks from `text` and return the remainder.
///
/// Some tags (e.g. `<system-reminder>`) are prepended as headers to real user
/// prompts by Claude Code's injection machinery. After stripping them the
/// actual human text remains. Returns `""` when no real content is left
/// (i.e., the turn is pure junk with no following human text).
fn strip_junk_prefixes(mut text: &str) -> &str {
    'outer: loop {
        let t = text.trim();
        for &prefix in JUNK_PREFIXES {
            if t.starts_with(prefix) {
                let tag_name = &prefix[1..prefix.len() - 1]; // "<foo>" → "foo"
                let close_tag = format!("</{tag_name}>");
                match t.find(close_tag.as_str()) {
                    Some(pos) => {
                        text = t[pos + close_tag.len()..].trim();
                        continue 'outer;
                    }
                    None => return "", // malformed / no closing tag — pure junk
                }
            }
        }
        break;
    }
    text.trim()
}

/// Plain-text prefix that marks a Claude Code session continuation summary.
/// These are injected by the runtime — not authored by the human.
const CONTINUATION_PREFIX: &str = "This session is being continued from a previous conversation";

/// Normalize raw user text: strip XML junk blocks, drop session continuations,
/// and compact skill invocations to a short readable label.
///
/// Returns `None` if the turn contains no real human content.
fn normalize_user_text(raw: &str) -> Option<String> {
    let text = strip_junk_prefixes(raw.trim());

    if text.is_empty() {
        return None;
    }

    if text.starts_with(CONTINUATION_PREFIX) {
        return None;
    }

    if let Some(rest) = text.strip_prefix("Base directory for this skill:") {
        let path = rest.lines().next().unwrap_or("").trim();
        let skill_name = path.split('/').next_back().unwrap_or("unknown");
        return Some(format!("Skill invocation: {skill_name}"));
    }

    Some(text.to_string())
}

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

/// Strip `project_root` from an absolute path, returning a repo-relative path.
///
/// If the path is already relative or doesn't start with `project_root`, it is
/// returned as-is. This normalizes the absolute paths that Claude Code writes
/// into tool `file_path` / `path` fields so they can be matched against
/// repo-relative `scope_files` from git.
fn relativize(path: &str, project_root: &Path) -> String {
    Path::new(path)
        .strip_prefix(project_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}

fn extract_from_session(
    path: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    project_root: &Path,
) -> Result<SessionExtractOutput> {
    let file = File::open(path).context("Failed to open session file")?;
    let reader = BufReader::new(file);

    let mut raw_messages: Vec<RawMessage> = Vec::new();
    let mut warnings = Vec::new();

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<RawMessage>(&line) {
            Ok(msg) => raw_messages.push(msg),
            Err(err) => warnings.push(format!(
                "{}:{} invalid JSON line skipped ({err})",
                path.display(),
                idx + 1
            )),
        }
    }

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

                        let (mut tool_calls, files_touched) =
                            collect_assistant_context(&raw_messages, i + 1);
                        if detect_skill_usage(&files_touched) {
                            push_unique(&mut tool_calls, "Skill");
                        }
                        let files_touched: Vec<String> = files_touched
                            .into_iter()
                            .map(|f| relativize(&f, project_root))
                            .collect();

                        let mut entry = PromptEntry::new(
                            branch,
                            String::new(),
                            prompt_text.clone(),
                            files_touched,
                            tool_calls,
                            "claude-code".to_string(),
                            None,
                        );
                        entry.assistant_context = extract_preceding_context(&raw_messages, i);
                        entries.push(with_timestamp(entry, ts));
                    }
                }
            }
        }

        i += 1;
    }

    Ok(SessionExtractOutput { entries, warnings })
}

/// Extract plain text from a user message.
fn extract_user_text(msg: &RawMessage) -> Option<String> {
    let body = msg.message.as_ref()?;
    if body.role.as_deref() != Some("user") {
        return None;
    }
    let content = body.content.as_ref()?;

    match content {
        Value::String(s) => normalize_user_text(s),
        Value::Array(parts) => {
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

            normalize_user_text(&text)
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
fn extract_preceding_context(messages: &[RawMessage], before_idx: usize) -> Option<String> {
    for msg in messages[..before_idx].iter().rev() {
        if msg.msg_type == "user" {
            break;
        }
        if msg.msg_type == "assistant" {
            if let Some(text) = extract_assistant_text(msg) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
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

/// Returns `true` if this `user` message is purely an API tool-result acknowledgment,
/// not a human-authored message.
///
/// In Claude Code's strict alternating-turn model every assistant `tool_use` must be
/// immediately followed by a user `tool_result` before the assistant can act again.
/// These intermediate turns are internal plumbing — they contain no human content.
fn is_tool_result_turn(msg: &RawMessage) -> bool {
    let content = match msg.message.as_ref().and_then(|b| b.content.as_ref()) {
        Some(c) => c,
        None => return false,
    };
    match content {
        Value::Array(parts) => {
            !parts.is_empty()
                && parts
                    .iter()
                    .all(|p| p.get("type").and_then(|v| v.as_str()) == Some("tool_result"))
        }
        _ => false,
    }
}

/// Walk forward from `start` collecting tool names and file paths from all
/// assistant turns belonging to this prompt's agentic response.
///
/// Skips over `tool_result` user turns (API plumbing) and stops only when a
/// real human message is encountered — allowing the full tool call chain from
/// multi-step agentic sessions to be captured.
fn collect_assistant_context(messages: &[RawMessage], start: usize) -> (Vec<String>, Vec<String>) {
    let mut tool_calls = Vec::new();
    let mut files_touched = Vec::new();

    for msg in messages[start..].iter() {
        if msg.msg_type == "user" {
            if is_tool_result_turn(msg) {
                continue;
            }
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
                        if name == "bash" {
                            let cmd = part.get("input").and_then(extract_command_from_tool_input);
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
                        } else {
                            let tool_name = normalize_tool_name(name);
                            push_unique(&mut tool_calls, &tool_name);
                        }

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
    match raw {
        "str_replace_based_edit_tool" | "write_file" => "Write",
        "bash" => "Bash",
        "read_file" => "Read",
        "list_directory" | "search_files" | "glob_files" => "Explore",
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
    match input.get("command")? {
        Value::String(s) => Some(s.to_string()),
        Value::Array(parts) => Some(
            parts
                .iter()
                .filter_map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => None,
    }
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
        "str_replace_based_edit_tool" | "write_file" | "read_file" | "Edit" | "Write" | "Read" => {
            input
                .get("file_path")
                .or_else(|| input.get("path"))
                .and_then(|v| v.as_str())
        }
        "bash" | "Bash" => None,
        _ => input
            .get("path")
            .or_else(|| input.get("file_path"))
            .and_then(|v| v.as_str()),
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
    use chrono::TimeZone;
    use std::io::Write;

    #[test]
    fn test_is_available_false_for_nonexistent_project() {
        let fake = Path::new("/tmp/nonexistent-project-xyz");
        assert!(!ClaudeCodeExtractor::is_available(fake));
    }

    #[test]
    fn test_normalize_tool_names() {
        assert_eq!(normalize_tool_name("bash"), "Bash");
        assert_eq!(normalize_tool_name("str_replace_based_edit_tool"), "Write");
        assert_eq!(normalize_tool_name("read_file"), "Read");
    }

    #[test]
    fn test_relativize_strips_project_root() {
        let root = Path::new("/Users/alice/myproject");
        assert_eq!(
            relativize("/Users/alice/myproject/src/main.rs", root),
            "src/main.rs"
        );
        assert_eq!(relativize("src/main.rs", root), "src/main.rs");
        assert_eq!(
            relativize("/Users/alice/otherproject/foo.rs", root),
            "/Users/alice/otherproject/foo.rs"
        );
    }

    #[test]
    fn test_strip_junk_drops_standalone_junk() {
        assert_eq!(
            strip_junk_prefixes("<command-name>/exit</command-name>"),
            ""
        );
        assert_eq!(
            strip_junk_prefixes("<local-command-stdout>Catch you later!</local-command-stdout>"),
            ""
        );
        assert_eq!(
            strip_junk_prefixes("<task-notification>task done</task-notification>"),
            ""
        );
    }

    #[test]
    fn test_strip_junk_preserves_real_prompt_after_system_reminder() {
        let input =
            "<system-reminder>\nsome injected context\n</system-reminder>\n\nfix the auth bug";
        assert_eq!(strip_junk_prefixes(input), "fix the auth bug");
    }

    #[test]
    fn test_strip_junk_passthrough_real_prompts() {
        assert_eq!(
            strip_junk_prefixes("fix the authentication bug"),
            "fix the authentication bug"
        );
        assert_eq!(strip_junk_prefixes(""), "");
    }

    #[test]
    fn test_normalize_drops_session_continuation() {
        let input = "This session is being continued from a previous conversation that ran out of context. The conversation is summarized below:\n...";
        assert_eq!(normalize_user_text(input), None);
    }

    #[test]
    fn test_normalize_compacts_skill_invocation() {
        let input = "Base directory for this skill: /Users/alice/.claude/skills/prompt-history\n\n# PromptEx — full skill context...";
        assert_eq!(
            normalize_user_text(input),
            Some("Skill invocation: prompt-history".to_string())
        );
    }

    fn raw_assistant_tool_use(tool_name: &str, file_path: Option<&str>) -> RawMessage {
        let mut input = serde_json::json!({});
        if let Some(p) = file_path {
            input["path"] = serde_json::Value::String(p.to_string());
        }
        RawMessage {
            msg_type: "assistant".to_string(),
            message: Some(MessageBody {
                role: Some("assistant".to_string()),
                content: Some(serde_json::json!([{
                    "type": "tool_use",
                    "name": tool_name,
                    "input": input
                }])),
            }),
            git_branch: None,
            timestamp: None,
        }
    }

    fn raw_tool_result_turn() -> RawMessage {
        RawMessage {
            msg_type: "user".to_string(),
            message: Some(MessageBody {
                role: Some("user".to_string()),
                content: Some(serde_json::json!([{
                    "type": "tool_result",
                    "tool_use_id": "toolu_123",
                    "content": "ok"
                }])),
            }),
            git_branch: None,
            timestamp: None,
        }
    }

    fn raw_human_turn(text: &str) -> RawMessage {
        RawMessage {
            msg_type: "user".to_string(),
            message: Some(MessageBody {
                role: Some("user".to_string()),
                content: Some(serde_json::Value::String(text.to_string())),
            }),
            git_branch: None,
            timestamp: None,
        }
    }

    #[test]
    fn test_collect_assistant_context_walks_through_tool_results() {
        let messages = vec![
            raw_assistant_tool_use("bash", None),
            raw_tool_result_turn(),
            raw_assistant_tool_use("str_replace_based_edit_tool", Some("src/foo.rs")),
            raw_tool_result_turn(),
            raw_assistant_tool_use("read_file", Some("src/bar.rs")),
            raw_tool_result_turn(),
            raw_human_turn("looks good"),
        ];

        let (tool_calls, files_touched) = collect_assistant_context(&messages, 0);

        assert!(tool_calls.contains(&"Bash".to_string()), "missing Bash");
        assert!(tool_calls.contains(&"Write".to_string()), "missing Write");
        assert!(tool_calls.contains(&"Read".to_string()), "missing Read");
        assert!(
            files_touched.contains(&"src/foo.rs".to_string()),
            "missing foo.rs"
        );
        assert!(
            files_touched.contains(&"src/bar.rs".to_string()),
            "missing bar.rs"
        );
    }

    #[test]
    fn test_collect_assistant_context_stops_at_human_turn() {
        let messages = vec![
            raw_assistant_tool_use("bash", None),
            raw_human_turn("next prompt"),
            raw_assistant_tool_use("read_file", Some("src/other.rs")),
        ];

        let (tool_calls, files_touched) = collect_assistant_context(&messages, 0);

        assert_eq!(tool_calls, vec!["Bash"]);
        assert!(files_touched.is_empty());
    }

    #[test]
    fn test_extract_file_copilot_names_and_field() {
        let edit_input = serde_json::json!({
            "file_path": "src/auth.rs",
            "old_string": "x",
            "new_string": "y"
        });
        assert_eq!(
            extract_file_from_tool("Write", Some(&edit_input)),
            Some("src/auth.rs".to_string())
        );

        let read_input = serde_json::json!({"file_path": "src/main.rs"});
        assert_eq!(
            extract_file_from_tool("Read", Some(&read_input)),
            Some("src/main.rs".to_string())
        );

        let cli_input = serde_json::json!({"path": "src/lib.rs"});
        assert_eq!(
            extract_file_from_tool("str_replace_based_edit_tool", Some(&cli_input)),
            Some("src/lib.rs".to_string())
        );
    }

    #[test]
    fn test_extract_reports_bad_jsonl_lines() {
        let log_dir = tempfile::TempDir::new().unwrap();
        let session = log_dir.path().join("session-1.jsonl");
        let mut f = std::fs::File::create(&session).unwrap();

        writeln!(f, "not-json").unwrap();
        writeln!(
            f,
            r#"{{"type":"user","timestamp":"2026-03-01T23:47:54Z","gitBranch":"main","message":{{"role":"user","content":"extract diagnostics"}}}}"#
        )
        .unwrap();

        let extractor =
            ClaudeCodeExtractor::new(log_dir.path().to_path_buf(), log_dir.path().to_path_buf());
        let since = Utc.with_ymd_and_hms(2026, 3, 1, 23, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 2, 0, 0, 0).unwrap();

        let output = extractor.extract(since, until).unwrap();
        assert_eq!(output.entries.len(), 1);
        assert_eq!(output.entries[0].prompt, "extract diagnostics");
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("invalid JSON line skipped"));
        assert!(output.warnings[0].contains("session-1.jsonl"));
    }
}
