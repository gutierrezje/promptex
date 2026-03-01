//! Extractor for OpenAI Codex CLI and desktop app session logs.
//!
//! Both the CLI and the Codex desktop app (released Jan 2026) share the same
//! `RolloutRecorder` from `codex-core` and write to the same path:
//!   ~/.codex/sessions/YYYY/MM/DD/rollout-{timestamp}-{uuid}.jsonl
//!
//! File structure (JSONL, adjacent-tagged `RolloutItem` enum):
//!   Line 0: `{"type": "session_meta", "payload": { id, timestamp, cwd, model_provider, ... }}`
//!   Line 1+: mixed events, notably:
//!     - `{"type":"event_msg","payload":{"type":"user_message", ...}}`
//!     - `{"type":"response_item","payload":{"type":"function_call"|"custom_tool_call", ...}}`
//!
//! Unlike Claude Code and OpenCode, timestamps are session-level only (line 0).
//! All entries from a session share the session start timestamp.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
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
        // Respects CODEX_HOME env var override (same as the CLI itself)
        let base = if let Ok(home) = std::env::var("CODEX_HOME") {
            PathBuf::from(home)
        } else {
            home_dir()?.join(".codex")
        };
        let dir = base.join("sessions");
        if dir.exists() {
            Some(dir)
        } else {
            None
        }
    }
}

impl PromptExtractor for CodexExtractor {
    fn is_available(_project_root: &Path) -> bool {
        Self::default_sessions_dir().is_some()
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<JournalEntry>> {
        let mut entries = Vec::new();

        for file in collect_jsonl_files(&self.sessions_dir) {
            let mut file_entries = extract_from_rollout(&file, since, until).unwrap_or_default();
            entries.append(&mut file_entries);
        }

        entries.sort_by_key(|e| e.timestamp);
        Ok(entries)
    }
}

// ── File collection ────────────────────────────────────────────────────────────

fn collect_jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_jsonl_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }

    files.sort();
    files
}

// ── Session extraction ─────────────────────────────────────────────────────────

fn extract_from_rollout(
    path: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Result<Vec<JournalEntry>> {
    let file = File::open(path).context("Failed to open Codex session file")?;
    let reader = BufReader::new(file);

    let mut lines: Vec<Value> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&line) {
            lines.push(v);
        }
    }

    if lines.is_empty() {
        return Ok(Vec::new());
    }

    // Line 0 is always session_meta — session-level timestamp and model.
    let session_ts = match parse_session_timestamp(&lines[0], path) {
        Some(ts) => ts,
        None => return Ok(Vec::new()),
    };

    if session_ts < since || session_ts > until {
        return Ok(Vec::new());
    }

    let model = extract_session_model(&lines[0]);

    // Walk subsequent lines, grouping events into turns by user_message boundaries.
    let mut entries = Vec::new();
    let mut i = 1;

    while i < lines.len() {
        if let Some(prompt) = extract_user_message(&lines[i]) {
            let (tool_calls, files_touched) = collect_turn_tools(&lines, i + 1);

            let entry = JournalEntry::new(
                "unknown".to_string(), // Codex sessions don't embed the git branch
                String::new(),
                prompt,
                files_touched,
                tool_calls,
                String::new(),
                "codex".to_string(),
                model.clone(),
            );
            entries.push(with_timestamp(entry, session_ts));
        }

        i += 1;
    }

    Ok(entries)
}

// ── SessionMeta parsing ────────────────────────────────────────────────────────

/// Parse the session timestamp from the `session_meta` payload, falling back to
/// the filename when the field is absent.
///
/// Filename format: `rollout-2026-02-03T13-40-28-{uuid}.jsonl`
/// Colons in the time portion are encoded as dashes to stay filesystem-safe.
fn parse_session_timestamp(meta: &Value, path: &Path) -> Option<DateTime<Utc>> {
    // Primary: session_meta payload.timestamp (RFC-3339 string)
    if let Some(ts) = meta
        .get("payload")
        .and_then(|p| p.get("timestamp"))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
    {
        return Some(ts);
    }

    // Fallback: parse from filename stem.
    // "rollout-2026-02-03T13-40-28-019c24ce-590a-7e42-b2e3-efe508ee3731"
    // NOTE: Codex writes the filename in LOCAL time (not UTC). Without a
    // timezone offset we cannot reliably convert, so this fallback is only
    // used when payload.timestamp is absent. The payload path is always
    // preferred and covers all known Codex versions.
    let stem = path.file_stem()?.to_str()?;
    let rest = stem.strip_prefix("rollout-")?;

    if rest.len() < 19 {
        return None;
    }

    let date = &rest[..10]; // "2026-02-03"
    let time = &rest[11..19]; // "13-40-28" (dashes instead of colons)

    // Parse as naive local datetime and convert to UTC via the system offset.
    // This is best-effort; rely on payload.timestamp when available.
    let naive_str = format!("{} {}", date, time.replace('-', ":"));
    chrono::NaiveDateTime::parse_from_str(&naive_str, "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|naive| {
            use chrono::TimeZone;
            chrono::Local.from_local_datetime(&naive).single()
        })
        .map(|local| local.with_timezone(&Utc))
}

fn extract_session_model(meta: &Value) -> Option<String> {
    meta.get("payload")
        .and_then(|p| p.get("model_provider"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

// ── Event parsing ──────────────────────────────────────────────────────────────

/// Extract the prompt text from a `user_message` event_msg line.
fn extract_user_message(event: &Value) -> Option<String> {
    if event.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = event.get("payload")?;
    if payload.get("type")?.as_str()? != "user_message" {
        return None;
    }
    // Codex Desktop v0.107+ uses "message" (string); older builds used "content"
    // (string or array of {type, text} parts). Try both.
    let content = payload.get("message").or_else(|| payload.get("content"))?;
    match content {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
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
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

/// Collect tool call names and touched files for the turn following a user message.
/// Stops at the next `user_message` event.
fn collect_turn_tools(events: &[Value], start: usize) -> (Vec<String>, Vec<String>) {
    let mut tool_calls: Vec<String> = Vec::new();
    let mut files_touched: Vec<String> = Vec::new();

    for event in &events[start..] {
        if extract_user_message(event).is_some() {
            break;
        }

        match event.get("type").and_then(|v| v.as_str()) {
            Some("response_item") => {
                collect_response_item_tool_event(event, &mut tool_calls, &mut files_touched)
            }
            Some("event_msg") => {
                collect_event_msg_tool_event(event, &mut tool_calls, &mut files_touched)
            }
            _ => {}
        }
    }

    (tool_calls, files_touched)
}

fn collect_response_item_tool_event(
    event: &Value,
    tool_calls: &mut Vec<String>,
    files_touched: &mut Vec<String>,
) {
    let Some(payload) = event.get("payload") else {
        return;
    };
    let Some(item_type) = payload.get("type").and_then(|v| v.as_str()) else {
        return;
    };

    match item_type {
        "function_call" => {
            let Some(name) = payload.get("name").and_then(|v| v.as_str()) else {
                return;
            };
            push_unique(tool_calls, &normalize_tool_name(name));

            let args = parse_embedded_json(payload.get("arguments"));
            if let Some(ref args_value) = args {
                collect_paths_from_named_fields(args_value, files_touched);
            }

            if let Some(cmd) = args
                .as_ref()
                .and_then(|v| v.get("cmd"))
                .and_then(|v| v.as_str())
            {
                extract_paths_from_command(cmd, files_touched);
            }

            if name == "parallel" {
                extract_parallel_inner_calls(args.as_ref(), tool_calls, files_touched);
            }
        }
        "custom_tool_call" => {
            let Some(name) = payload.get("name").and_then(|v| v.as_str()) else {
                return;
            };
            push_unique(tool_calls, &normalize_tool_name(name));

            if name == "apply_patch" {
                if let Some(input) = payload.get("input").and_then(|v| v.as_str()) {
                    extract_paths_from_apply_patch(input, files_touched);
                }
            }
        }
        _ => {}
    }
}

fn collect_event_msg_tool_event(
    event: &Value,
    tool_calls: &mut Vec<String>,
    files_touched: &mut Vec<String>,
) {
    let Some(payload) = event.get("payload") else {
        return;
    };
    let Some(event_type) = payload.get("type").and_then(|v| v.as_str()) else {
        return;
    };

    match event_type {
        "exec_command_begin" => {
            push_unique(tool_calls, "Bash");
        }
        "mcp_tool_call_begin" => {
            if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                push_unique(tool_calls, &normalize_tool_name(name));
            }
            if let Some(file) = payload
                .get("arguments")
                .and_then(|a| a.get("path").or_else(|| a.get("file_path")))
                .and_then(|v| v.as_str())
            {
                push_path(files_touched, file);
            }
        }
        "apply_patch_approval_request" => {
            push_unique(tool_calls, "Patch");
        }
        _ => {}
    }
}

fn parse_embedded_json(value: Option<&Value>) -> Option<Value> {
    match value {
        Some(Value::String(s)) => serde_json::from_str::<Value>(s).ok(),
        Some(v @ Value::Object(_)) | Some(v @ Value::Array(_)) => Some(v.clone()),
        _ => None,
    }
}

fn extract_parallel_inner_calls(
    args: Option<&Value>,
    tool_calls: &mut Vec<String>,
    files_touched: &mut Vec<String>,
) {
    let Some(tool_uses) = args
        .and_then(|v| v.get("tool_uses"))
        .and_then(|v| v.as_array())
    else {
        return;
    };

    for tool_use in tool_uses {
        let Some(recipient_name) = tool_use.get("recipient_name").and_then(|v| v.as_str()) else {
            continue;
        };
        let inner_name = recipient_name.rsplit('.').next().unwrap_or(recipient_name);
        push_unique(tool_calls, &normalize_tool_name(inner_name));

        if let Some(params) = tool_use.get("parameters") {
            collect_paths_from_named_fields(params, files_touched);
            if let Some(cmd) = params.get("cmd").and_then(|v| v.as_str()) {
                extract_paths_from_command(cmd, files_touched);
            }
        }
    }
}

fn collect_paths_from_named_fields(value: &Value, files_touched: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                match key.as_str() {
                    "path" | "file_path" | "filepath" | "file" | "filename" => match child {
                        Value::String(s) => push_path(files_touched, s),
                        Value::Array(arr) => {
                            for item in arr {
                                if let Some(s) = item.as_str() {
                                    push_path(files_touched, s);
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => collect_paths_from_named_fields(child, files_touched),
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_paths_from_named_fields(item, files_touched);
            }
        }
        _ => {}
    }
}

fn extract_paths_from_apply_patch(input: &str, files_touched: &mut Vec<String>) {
    for line in input.lines() {
        for prefix in [
            "*** Update File: ",
            "*** Add File: ",
            "*** Delete File: ",
            "*** Move to: ",
        ] {
            if let Some(path) = line.strip_prefix(prefix) {
                push_path(files_touched, path.trim());
            }
        }
    }
}

fn extract_paths_from_command(cmd: &str, files_touched: &mut Vec<String>) {
    for raw in cmd.split_whitespace() {
        let token = raw.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        });
        if token.is_empty() {
            continue;
        }
        if is_path_like_token(token) {
            push_path(files_touched, token);
        }
    }
}

fn push_path(files_touched: &mut Vec<String>, candidate: &str) {
    let path = candidate.trim();
    if path.is_empty() {
        return;
    }
    if path.starts_with("http://") || path.starts_with("https://") {
        return;
    }
    push_unique(files_touched, path);
}

fn is_path_like_token(token: &str) -> bool {
    if token.starts_with('-') {
        return false;
    }
    if matches!(token, "|" | "||" | "&&" | ">" | ">>" | "<" | "<<" | ";") {
        return false;
    }
    if token.starts_with("~/")
        || token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with('/')
        || token.contains('/')
    {
        return true;
    }

    // Bare filenames like Cargo.toml / src.rs.
    if let Some((stem, ext)) = token.rsplit_once('.') {
        let stem_has_alpha = stem.chars().any(|c| c.is_ascii_alphabetic());
        let ext_has_alpha = ext.chars().any(|c| c.is_ascii_alphabetic());
        let valid_chars = token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '*' | '?'));
        return stem_has_alpha && ext_has_alpha && valid_chars;
    }

    false
}

fn normalize_tool_name(name: &str) -> String {
    match name {
        "edit" | "write_file" | "create_file" => "Edit",
        "read" | "read_file" | "view" | "open" => "Read",
        "bash" | "shell" | "exec_command" | "write_stdin" => "Bash",
        "apply_patch" => "Patch",
        other => other,
    }
    .to_string()
}

fn push_unique(vec: &mut Vec<String>, item: &str) {
    let s = item.to_string();
    if !vec.contains(&s) {
        vec.push(s);
    }
}

fn with_timestamp(mut entry: JournalEntry, ts: DateTime<Utc>) -> JournalEntry {
    entry.timestamp = ts;
    entry
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::io::Write;

    #[test]
    fn test_collect_jsonl_files_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(collect_jsonl_files(dir.path()).is_empty());
    }

    #[test]
    fn test_is_available_checks_directory() {
        let _ = CodexExtractor::is_available(Path::new("/tmp"));
    }

    #[test]
    fn test_parse_timestamp_from_payload() {
        let meta = serde_json::json!({
            "type": "session_meta",
            "payload": {"timestamp": "2026-02-03T13:40:28Z"}
        });
        let ts = parse_session_timestamp(&meta, Path::new("irrelevant.jsonl")).unwrap();
        assert_eq!(
            ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-02-03 13:40:28"
        );
    }

    #[test]
    fn test_parse_timestamp_from_filename_fallback() {
        // Filename uses local time; the fallback converts to UTC via the system
        // offset. We can't assert a fixed UTC string here because the offset
        // varies by machine, so just verify the parsed result is within ±14h of
        // the naive time (a valid UTC offset range).
        let meta = serde_json::json!({"type": "session_meta", "payload": {}});
        let path = Path::new("rollout-2026-02-03T13-40-28-019c24ce.jsonl");
        let ts = parse_session_timestamp(&meta, path).unwrap();
        let naive =
            chrono::NaiveDateTime::parse_from_str("2026-02-03 13:40:28", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        let diff = (ts.naive_utc() - naive).num_hours().abs();
        assert!(diff <= 14, "UTC offset should be within ±14h, got {diff}h");
    }

    #[test]
    fn test_extract_user_message_correct_format() {
        let event = serde_json::json!({
            "type": "event_msg",
            "payload": {"type": "user_message", "content": "fix the auth bug"}
        });
        assert_eq!(
            extract_user_message(&event),
            Some("fix the auth bug".to_string())
        );
    }

    #[test]
    fn test_extract_user_message_rejects_old_role_format() {
        let event = serde_json::json!({
            "type": "message",
            "payload": {"role": "user", "content": "old format"}
        });
        assert_eq!(extract_user_message(&event), None);
    }

    #[test]
    fn test_normalize_tool_name() {
        assert_eq!(normalize_tool_name("edit"), "Edit");
        assert_eq!(normalize_tool_name("read_file"), "Read");
        assert_eq!(normalize_tool_name("bash"), "Bash");
        assert_eq!(normalize_tool_name("exec_command"), "Bash");
        assert_eq!(normalize_tool_name("apply_patch"), "Patch");
        assert_eq!(normalize_tool_name("my_custom_tool"), "my_custom_tool");
    }

    #[test]
    fn test_full_rollout_extraction() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","originator":"Codex Desktop","source":"appServer","cli_version":"1.0","model_provider":"openai/codex-mini"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"add auth validation"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"mcp_tool_call_begin","name":"edit","arguments":{{"path":"src/auth.rs"}}}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"turn_complete"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"run tests"}}}}"#
        )
        .unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"exec_command_begin","command":["cargo","test"]}}}}"#).unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 15, 11, 0, 0).unwrap();
        let entries = extract_from_rollout(&path, since, until).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].prompt, "add auth validation");
        assert_eq!(entries[0].tool_calls, vec!["Edit"]);
        assert_eq!(entries[0].files_touched, vec!["src/auth.rs"]);
        assert_eq!(entries[0].tool, "codex");
        assert_eq!(entries[0].model, Some("openai/codex-mini".to_string()));
        assert_eq!(entries[1].prompt, "run tests");
        assert_eq!(entries[1].tool_calls, vec!["Bash"]);
    }

    #[test]
    fn test_response_item_tool_calls_and_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-03-01T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-01T21:56:17Z","cwd":"/proj","originator":"Codex Desktop","source":"vscode","model_provider":"openai"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"add a one-line comment"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"response_item","payload":{{"type":"function_call","name":"exec_command","arguments":"{{\"cmd\":\"sed -n '1,40p' Cargo.toml\"}}","call_id":"c1"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"response_item","payload":{{"type":"custom_tool_call","status":"completed","call_id":"c2","name":"apply_patch","input":"*** Begin Patch\n*** Update File: Cargo.toml\n*** End Patch\n"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"second prompt"}}}}"#).unwrap();

        let since = Utc.with_ymd_and_hms(2026, 3, 1, 20, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 1, 23, 0, 0).unwrap();
        let entries = extract_from_rollout(&path, since, until).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].prompt, "add a one-line comment");
        assert_eq!(entries[0].tool_calls, vec!["Bash", "Patch"]);
        assert_eq!(entries[0].files_touched, vec!["Cargo.toml"]);
        assert_eq!(entries[1].prompt, "second prompt");
    }

    #[test]
    fn test_session_outside_time_range_skipped() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","originator":"cli","source":"cli","cli_version":"1.0"}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"some prompt"}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 16, 0, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 17, 0, 0, 0).unwrap();
        let entries = extract_from_rollout(&path, since, until).unwrap();

        assert!(entries.is_empty());
    }
}
