//! Extract prompts from Codex CLI and desktop rollout logs.
//!
//! Codex stores JSONL session files under `~/.codex/sessions/`. The first line
//! contains session metadata; later lines contain user messages, tool calls,
//! and other events. This extractor timestamps prompts from the `user_message`
//! event when possible and falls back to session metadata.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use super::ExtractorOutput;
use crate::prompt::PromptEntry;

/// Extracts Codex CLI/Desktop rollout logs for a single project.
pub struct CodexExtractor {
    sessions_dir: PathBuf,
    project_root: PathBuf,
}

impl CodexExtractor {
    /// Create a Codex extractor for the given sessions directory and project root.
    pub fn new(sessions_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            sessions_dir,
            project_root,
        }
    }

    /// Resolve the default Codex sessions directory, if present.
    pub fn default_sessions_dir() -> Option<PathBuf> {
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

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<ExtractorOutput> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        let project_root = canonicalize_dir(&self.project_root);

        for file in collect_jsonl_files(&self.sessions_dir) {
            match extract_from_rollout(&file, since, until, &project_root) {
                Ok(mut rollout_out) => {
                    entries.append(&mut rollout_out.entries);
                    warnings.append(&mut rollout_out.warnings);
                }
                Err(err) => {
                    warnings.push(format!(
                        "failed to extract rollout from {}: {err}",
                        file.display()
                    ));
                }
            }
        }

        entries.sort_by_key(|e| e.timestamp);
        Ok(ExtractorOutput { entries, warnings })
    }
}

struct RolloutExtractOutput {
    entries: Vec<PromptEntry>,
    warnings: Vec<String>,
}

const MAX_CONTEXT_CHARS: usize = 300;

fn collect_jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_jsonl_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "jsonl")
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("rollout-"))
        {
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
    project_root: &Path,
) -> Result<RolloutExtractOutput> {
    let file = File::open(path).context("Failed to open Codex session file")?;
    let reader = BufReader::new(file);

    let mut lines: Vec<Value> = Vec::new();
    let mut warnings = Vec::new();

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(v) => lines.push(v),
            Err(err) => warnings.push(format!(
                "skipped invalid JSON line at {}:{}: {err}",
                path.display(),
                idx + 1
            )),
        }
    }

    if lines.is_empty() {
        return Ok(RolloutExtractOutput {
            entries: Vec::new(),
            warnings,
        });
    }

    let Some(session_meta) = find_session_meta(&lines) else {
        warnings.push(format!(
            "missing session_meta in {}: skipping rollout",
            path.display()
        ));
        return Ok(RolloutExtractOutput {
            entries: Vec::new(),
            warnings,
        });
    };

    match extract_session_cwd(session_meta) {
        Some(cwd) => {
            if !cwd.is_absolute() {
                warnings.push(format!(
                    "session cwd is not absolute in {}: skipping rollout",
                    path.display()
                ));
                return Ok(RolloutExtractOutput {
                    entries: Vec::new(),
                    warnings,
                });
            }
            let cwd = canonicalize_dir(&cwd);
            let project_root = canonicalize_dir(project_root);
            if cwd.strip_prefix(&project_root).is_err() {
                return Ok(RolloutExtractOutput {
                    entries: Vec::new(),
                    warnings,
                });
            }
        }
        None => {
            warnings.push(format!(
                "session cwd missing in {}: skipping rollout",
                path.display()
            ));
            return Ok(RolloutExtractOutput {
                entries: Vec::new(),
                warnings,
            });
        }
    }

    let session_ts_fallback = parse_session_timestamp(session_meta, path);
    let mut model = extract_session_model(session_meta);

    let mut entries = Vec::new();
    let mut i = 1;

    while i < lines.len() {
        if let Some((updated, is_specific)) = extract_model_from_event(&lines[i]) {
            if is_specific || model.is_none() {
                model = Some(updated);
            }
        }
        if let Some(prompt) = extract_user_message(&lines[i]) {
            let Some(prompt_ts) = parse_event_timestamp(&lines[i]).or(session_ts_fallback) else {
                i += 1;
                continue;
            };
            if prompt_ts < since || prompt_ts > until {
                i += 1;
                continue;
            }

            let (mut tool_calls, files_touched) = collect_turn_tools(&lines, i + 1);
            if detect_skill_usage(&files_touched) {
                push_unique(&mut tool_calls, "Skill");
            }
            let files_touched = normalize_files_touched(files_touched, project_root);

            let mut entry = PromptEntry::new(
                "unknown".to_string(),
                String::new(),
                prompt,
                files_touched,
                tool_calls,
                "codex".to_string(),
                model.clone(),
            );
            entry.assistant_context = extract_preceding_assistant_context(&lines, i);
            entries.push(with_timestamp(entry, prompt_ts));
        }

        i += 1;
    }

    Ok(RolloutExtractOutput { entries, warnings })
}

/// Parse the session timestamp from the `session_meta` payload, falling back to
/// the filename when the field is absent.
///
/// Filename format: `rollout-2026-02-03T13-40-28-{uuid}.jsonl`
/// Colons in the time portion are encoded as dashes to stay filesystem-safe.
fn parse_session_timestamp(meta: &Value, path: &Path) -> Option<DateTime<Utc>> {
    if let Some(ts) = meta
        .get("payload")
        .and_then(|p| p.get("timestamp"))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
    {
        return Some(ts);
    }

    // Filename timestamps are local time, so this is only a best-effort
    // fallback when the structured timestamp is missing.
    let stem = path.file_stem()?.to_str()?;
    let rest = stem.strip_prefix("rollout-")?;

    if rest.len() < 19 {
        return None;
    }

    let date = &rest[..10];
    let time = &rest[11..19];

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
        .and_then(extract_model_from_payload)
        .map(|(value, _)| value)
}

/// Returns (model_string, is_specific) where is_specific is false when
/// only model_provider matched (i.e., no concrete model identifier).
fn extract_model_from_event(event: &Value) -> Option<(String, bool)> {
    match event.get("type").and_then(|v| v.as_str()) {
        Some("session_meta") | Some("turn_context") => {
            event.get("payload").and_then(extract_model_from_payload)
        }
        _ => None,
    }
}

/// Returns (value, is_specific). `is_specific` is true when the value came
/// from a model-identity key (model, model_name, model_slug, model_id)
/// rather than the fallback model_provider key.
fn extract_model_from_payload(payload: &Value) -> Option<(String, bool)> {
    for key in ["model", "model_name", "model_slug", "model_id"] {
        if let Some(value) = payload.get(key).and_then(|v| v.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some((trimmed.to_string(), true));
            }
        }
    }
    if let Some(value) = payload.get("model_provider").and_then(|v| v.as_str()) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some((trimmed.to_string(), false));
        }
    }
    None
}

fn extract_session_cwd(meta: &Value) -> Option<PathBuf> {
    meta.get("payload")
        .and_then(|p| p.get("cwd"))
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
}

fn find_session_meta(lines: &[Value]) -> Option<&Value> {
    lines
        .iter()
        .find(|line| line.get("type").and_then(|v| v.as_str()) == Some("session_meta"))
}

fn canonicalize_dir(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn parse_event_timestamp(event: &Value) -> Option<DateTime<Utc>> {
    event
        .get("timestamp")
        .and_then(|v| v.as_str())
        .or_else(|| {
            event
                .get("payload")
                .and_then(|p| p.get("timestamp"))
                .and_then(|v| v.as_str())
        })
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn extract_preceding_assistant_context(events: &[Value], before_idx: usize) -> Option<String> {
    for event in events[..before_idx].iter().rev() {
        if extract_user_message(event).is_some() {
            break;
        }
        if let Some(text) = extract_assistant_text(event) {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let count = trimmed.chars().count();
            let tail: String = if count <= MAX_CONTEXT_CHARS {
                trimmed.to_string()
            } else {
                trimmed.chars().skip(count - MAX_CONTEXT_CHARS).collect()
            };
            return Some(tail);
        }
    }
    None
}

fn extract_assistant_text(event: &Value) -> Option<String> {
    match event.get("type").and_then(|v| v.as_str()) {
        Some("response_item") => {
            let payload = event.get("payload")?;
            extract_response_item_text(payload)
        }
        Some("event_msg") => {
            let payload = event.get("payload")?;
            extract_event_msg_text(payload)
        }
        _ => None,
    }
}

fn extract_response_item_text(payload: &Value) -> Option<String> {
    if has_non_assistant_role(payload) {
        return None;
    }
    let item_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match item_type {
        "message" | "assistant_message" | "output_text" => extract_text_value(
            payload
                .get("message")
                .or_else(|| payload.get("content"))
                .or_else(|| payload.get("text"))?,
        ),
        _ => None,
    }
}

fn extract_event_msg_text(payload: &Value) -> Option<String> {
    if has_non_assistant_role(payload) {
        return None;
    }
    let event_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match event_type {
        "assistant_message" | "assistant_response" | "output_text" => extract_text_value(
            payload
                .get("message")
                .or_else(|| payload.get("content"))
                .or_else(|| payload.get("text"))?,
        ),
        _ => None,
    }
}

fn has_non_assistant_role(payload: &Value) -> bool {
    // Codex assistant responses commonly omit the "role" field entirely.
    // Treat missing role as assistant (return false) so those messages are
    // not filtered out when extracting assistant context.
    let role = payload.get("role").and_then(|v| v.as_str()).or_else(|| {
        payload
            .get("message")
            .and_then(|m| m.get("role"))
            .and_then(|v| v.as_str())
    });
    matches!(role, Some(r) if r != "assistant")
}

fn extract_text_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Value::Array(parts) => {
            let text = parts
                .iter()
                .filter_map(extract_text_value)
                .collect::<Vec<_>>()
                .join(" ");
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Object(map) => {
            if let Some(v) = map
                .get("text")
                .or_else(|| map.get("content"))
                .or_else(|| map.get("message"))
            {
                return extract_text_value(v);
            }
            None
        }
        _ => None,
    }
}

/// Extract the prompt text from a `user_message` event_msg line.
fn extract_user_message(event: &Value) -> Option<String> {
    if event.get("type")?.as_str()? != "event_msg" {
        return None;
    }
    let payload = event.get("payload")?;
    if payload.get("type")?.as_str()? != "user_message" {
        return None;
    }
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

        let mut turn_tools = Vec::new();
        let mut turn_files = Vec::new();

        match event.get("type").and_then(|v| v.as_str()) {
            Some("response_item") => {
                collect_response_item_tool_event(event, &mut turn_tools, &mut turn_files)
            }
            Some("event_msg") => {
                collect_event_msg_tool_event(event, &mut turn_tools, &mut turn_files)
            }
            _ => {}
        }

        for tool in turn_tools {
            if !tool_calls.contains(&tool) {
                tool_calls.push(tool);
            }
        }
        for file in turn_files {
            if !files_touched.contains(&file) {
                files_touched.push(file);
            }
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
            let args = parse_embedded_json(payload.get("arguments"));
            let cmd_arg = args
                .as_ref()
                .and_then(|v| v.get("cmd"))
                .and_then(|v| v.as_str());

            push_tool_call(tool_calls, name, cmd_arg);

            if let Some(ref args_value) = args {
                collect_paths_from_named_fields(args_value, files_touched);
            }

            if let Some(cmd) = cmd_arg {
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
            push_tool_call(tool_calls, name, None);

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
            let cmd = payload.get("command").and_then(|v| match v {
                Value::String(s) => Some(s.to_string()),
                Value::Array(parts) => Some(
                    parts
                        .iter()
                        .filter_map(|p| p.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                _ => None,
            });
            if let Some(cmd_ref) = cmd.as_deref() {
                extract_paths_from_command(cmd_ref, files_touched);
            }
            push_command_tool_calls(tool_calls, cmd.as_deref());
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
            push_unique(tool_calls, "Write");
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

        if let Some(params) = tool_use.get("parameters") {
            let cmd = params.get("cmd").and_then(|v| v.as_str());
            push_tool_call(tool_calls, inner_name, cmd);
            collect_paths_from_named_fields(params, files_touched);
            if let Some(cmd) = params.get("cmd").and_then(|v| v.as_str()) {
                extract_paths_from_command(cmd, files_touched);
            }
        } else {
            push_tool_call(tool_calls, inner_name, None);
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
    let mut heredoc_end: Option<String> = None;

    for line in cmd.lines() {
        if let Some(end) = &heredoc_end {
            if line.trim() == end {
                heredoc_end = None;
            }
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;
        while i < tokens.len() {
            let raw = tokens[i];
            if raw == "<<" || raw == "<<-" {
                if let Some(next) = tokens.get(i + 1) {
                    heredoc_end = normalize_heredoc_delimiter(&format!("{raw}{next}"));
                    i += 2;
                    continue;
                }
            }

            if let Some(delim) = normalize_heredoc_delimiter(raw) {
                heredoc_end = Some(delim);
                i += 1;
                continue;
            }

            let token = raw.trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
                )
            });
            if token.is_empty() {
                i += 1;
                continue;
            }
            if is_path_like_token(token) {
                push_path(files_touched, token);
            }
            i += 1;
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
    if token == "/" {
        return false;
    }
    if token.contains('<')
        || token.contains('>')
        || token.contains("://")
        || token.contains('$')
        || token.contains('=')
    {
        return false;
    }
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

    // Allow bare filenames like Cargo.toml or main.rs.
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

fn normalize_heredoc_delimiter(token: &str) -> Option<String> {
    let mut t = token;
    if let Some(stripped) = t.strip_prefix("<<-") {
        t = stripped;
    } else if let Some(stripped) = t.strip_prefix("<<") {
        t = stripped;
    } else {
        return None;
    }

    let t = t.trim_matches(|c| c == '\'' || c == '"');
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn normalize_files_touched(files: Vec<String>, project_root: &Path) -> Vec<String> {
    let project_root = canonicalize_dir(project_root);
    let mut normalized = Vec::new();
    for file in files {
        let trimmed = file.trim();
        if trimmed.starts_with("~/") {
            continue;
        }
        let candidate = Path::new(trimmed);
        if candidate.is_absolute() {
            let candidate = canonicalize_dir(candidate);
            if let Ok(rel) = candidate.strip_prefix(&project_root) {
                let has_parent_dir = rel
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir));
                if !has_parent_dir {
                    push_unique(&mut normalized, &rel.to_string_lossy());
                }
            }
            continue;
        }
        let joined = project_root.join(trimmed);
        if let Ok(canonical_joined) = joined.canonicalize() {
            // Both paths resolve on disk — use canonical comparison
            let canonical_root = canonicalize_dir(&project_root);
            if let Ok(rel) = canonical_joined.strip_prefix(&canonical_root) {
                push_unique(&mut normalized, &rel.to_string_lossy());
            }
        } else {
            // Path doesn't exist on disk — use textual strip_prefix but
            // reject any result containing ".." components
            let joined_normalized = canonicalize_dir(&joined);
            if let Ok(rel) = joined_normalized.strip_prefix(&project_root) {
                let has_parent_dir = rel
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir));
                if !has_parent_dir {
                    push_unique(&mut normalized, &rel.to_string_lossy());
                }
            }
        }
    }
    normalized
}

fn normalize_tool_name(name: &str) -> String {
    match name {
        "edit" | "write_file" | "create_file" | "apply_patch" => "Write",
        "read" | "read_file" | "view" | "open" => "Read",
        "list_directory" | "search_files" | "glob_files" => "Explore",
        "bash" | "shell" | "exec_command" | "write_stdin" => "Bash",
        other => other,
    }
    .to_string()
}

fn push_tool_call(tool_calls: &mut Vec<String>, name: &str, cmd: Option<&str>) {
    if matches!(name, "exec_command" | "write_stdin" | "bash" | "shell") {
        push_command_tool_calls(tool_calls, cmd);
        return;
    }
    push_unique(tool_calls, &normalize_tool_name(name));
}

fn push_command_tool_calls(tool_calls: &mut Vec<String>, cmd: Option<&str>) {
    if let Some(cmd) = cmd {
        let inferred = infer_command_categories(cmd);
        if !inferred.is_empty() {
            for category in inferred {
                push_unique(tool_calls, &category);
            }
            return;
        }
    }
    push_unique(tool_calls, "Bash");
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
        // The exact UTC value depends on the machine's local timezone, so
        // assert a reasonable offset bound instead of a fixed timestamp.
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
    fn test_parse_event_timestamp_from_line() {
        let event = serde_json::json!({
            "timestamp": "2026-03-01T23:47:54.761Z",
            "type": "event_msg",
            "payload": {"type": "user_message", "message": "extract prompts"}
        });
        let ts = parse_event_timestamp(&event).unwrap();
        assert_eq!(
            ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-03-01 23:47:54"
        );
    }

    #[test]
    fn test_normalize_tool_name() {
        assert_eq!(normalize_tool_name("edit"), "Write");
        assert_eq!(normalize_tool_name("read_file"), "Read");
        assert_eq!(normalize_tool_name("bash"), "Bash");
        assert_eq!(normalize_tool_name("exec_command"), "Bash");
        assert_eq!(normalize_tool_name("apply_patch"), "Write");
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
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();
        let entries = output.entries;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].prompt, "add auth validation");
        assert_eq!(entries[0].tool_calls, vec!["Write"]);
        assert_eq!(entries[0].files_touched, vec!["src/auth.rs"]);
        assert_eq!(entries[0].tool, "codex");
        assert_eq!(entries[0].model, Some("openai/codex-mini".to_string()));
        assert_eq!(entries[1].prompt, "run tests");
        assert_eq!(entries[1].tool_calls, vec!["Bash"]);
    }

    #[test]
    fn test_collect_turn_tools_deduplicates_and_maps_canonical() {
        let events = vec![
            serde_json::json!({
                "type": "event_msg",
                "payload": {"type": "mcp_tool_call_begin", "name": "edit", "arguments": {"path": "src/a.rs"}}
            }),
            serde_json::json!({
                "type": "event_msg",
                "payload": {"type": "mcp_tool_call_begin", "name": "edit", "arguments": {"path": "src/a.rs"}}
            }),
            serde_json::json!({
                "type": "event_msg",
                "payload": {"type": "mcp_tool_call_begin", "name": "view", "arguments": {"path": "src/b.rs"}}
            }),
        ];

        let (tool_calls, files_touched) = collect_turn_tools(&events, 0);
        assert_eq!(tool_calls, vec!["Write", "Read"]);
        assert_eq!(files_touched, vec!["src/a.rs", "src/b.rs"]);
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
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();
        let entries = output.entries;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].prompt, "add a one-line comment");
        assert_eq!(entries[0].tool_calls, vec!["Read", "Write"]);
        assert_eq!(entries[0].files_touched, vec!["Cargo.toml"]);
        assert_eq!(entries[1].prompt, "second prompt");
    }

    #[test]
    fn test_normalizes_absolute_paths_under_project_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-03-01T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-01T21:56:17Z","cwd":"/proj","originator":"Codex Desktop","source":"vscode","model_provider":"openai"}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"touch file"}}}}"#
        )
        .unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"mcp_tool_call_begin","name":"edit","arguments":{{"path":"/proj/src/lib.rs"}}}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"second"}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2026, 3, 1, 20, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 1, 23, 0, 0).unwrap();
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();
        let entries = output.entries;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].files_touched, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_normalize_files_touched_drops_external_paths() {
        let files = vec![
            "/tmp/other.txt".to_string(),
            "~/secrets.txt".to_string(),
            "/proj/src/lib.rs".to_string(),
            "relative.txt".to_string(),
        ];
        let normalized = normalize_files_touched(files, Path::new("/proj"));
        assert_eq!(normalized, vec!["src/lib.rs", "relative.txt"]);
    }

    #[test]
    fn test_normalize_files_touched_drops_parent_dir_paths() {
        let files = vec![
            "../outside.txt".to_string(),
            "src/../../../escape.txt".to_string(),
            "valid/nested/file.rs".to_string(),
        ];
        let normalized = normalize_files_touched(files, Path::new("/proj"));
        assert_eq!(normalized, vec!["valid/nested/file.rs"]);
    }

    #[test]
    fn test_ignores_heredoc_body_tokens_in_commands() {
        let mut files = Vec::new();
        let cmd = "cat <<'EOF' > src/output.md\n## Prompt History\nfeature/codex-parity\nEOF\n";

        extract_paths_from_command(cmd, &mut files);

        assert_eq!(files, vec!["src/output.md"]);
    }

    #[test]
    fn test_uses_user_message_timestamp_for_filtering() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","originator":"cli","source":"cli"}}}}"#).unwrap();
        writeln!(f, r#"{{"timestamp":"2026-01-16T00:10:00Z","type":"event_msg","payload":{{"type":"user_message","content":"in range prompt"}}}}"#).unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 16, 0, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 16, 1, 0, 0).unwrap();
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();
        let entries = output.entries;

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].prompt, "in range prompt");
        assert_eq!(
            entries[0].timestamp,
            Utc.with_ymd_and_hms(2026, 1, 16, 0, 10, 0).unwrap()
        );
    }

    #[test]
    fn test_extract_captures_assistant_context() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","originator":"cli","source":"cli"}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"first prompt"}}}}"#
        )
        .unwrap();
        writeln!(f, r#"{{"type":"response_item","payload":{{"type":"message","content":"assistant reply"}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"second prompt"}}}}"#).unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 15, 11, 0, 0).unwrap();
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();

        assert_eq!(output.entries.len(), 2);
        assert_eq!(
            output.entries[1].assistant_context.as_deref(),
            Some("assistant reply")
        );
    }

    #[test]
    fn test_extract_prefers_turn_context_model() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","model_provider":"openai"}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"turn_context","payload":{{"model":"gpt-5.2-codex"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"first prompt"}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 15, 11, 0, 0).unwrap();
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();

        assert_eq!(output.entries.len(), 1);
        assert_eq!(output.entries[0].model.as_deref(), Some("gpt-5.2-codex"));
    }

    #[test]
    fn test_extract_ignores_non_assistant_role_for_context() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","originator":"cli","source":"cli"}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"first prompt"}}}}"#
        )
        .unwrap();
        writeln!(f, r#"{{"type":"response_item","payload":{{"type":"message","message":{{"role":"user","content":"user echo"}}}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"second prompt"}}}}"#).unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 15, 11, 0, 0).unwrap();
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();

        assert_eq!(output.entries.len(), 2);
        assert!(output.entries[1].assistant_context.is_none());
    }

    #[test]
    fn test_extract_assistant_context_from_message_object() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir
            .path()
            .join("rollout-2026-01-15T10-00-00-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-01-15T10:00:00Z","cwd":"/proj","originator":"cli","source":"cli"}}}}"#).unwrap();
        writeln!(
            f,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"first prompt"}}}}"#
        )
        .unwrap();
        writeln!(f, r#"{{"type":"response_item","payload":{{"type":"message","message":{{"role":"assistant","content":[{{"type":"output_text","text":"nested reply"}}]}}}}}}"#).unwrap();
        writeln!(f, r#"{{"type":"event_msg","payload":{{"type":"user_message","content":"second prompt"}}}}"#).unwrap();

        let since = Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 1, 15, 11, 0, 0).unwrap();
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();

        assert_eq!(output.entries.len(), 2);
        assert_eq!(
            output.entries[1].assistant_context.as_deref(),
            Some("nested reply")
        );
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
        let output = extract_from_rollout(&path, since, until, Path::new("/proj")).unwrap();
        let entries = output.entries;

        assert!(entries.is_empty());
    }

    #[test]
    fn test_extract_reports_bad_jsonl_lines() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("2026").join("03").join("01");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("rollout-2026-03-01T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(f, "{{not-json").unwrap();
        writeln!(f, r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-01T21:56:17Z","cwd":"/proj","originator":"Codex Desktop","source":"vscode","model_provider":"openai"}}}}"#).unwrap();
        writeln!(f, r#"{{"timestamp":"2026-03-01T21:58:00Z","type":"event_msg","payload":{{"type":"user_message","message":"collect diagnostics"}}}}"#).unwrap();

        let extractor = CodexExtractor::new(dir.path().to_path_buf(), PathBuf::from("/proj"));
        let since = Utc.with_ymd_and_hms(2026, 3, 1, 21, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 1, 23, 0, 0).unwrap();

        let output = extractor.extract(since, until).unwrap();
        assert_eq!(output.entries.len(), 1);
        assert_eq!(output.entries[0].prompt, "collect diagnostics");
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("skipped invalid JSON line at"));
        assert!(output.warnings[0].contains("rollout-2026-03-01T13-56-17-testuuid.jsonl"));

        crate::extractors::test_contract::assert_entries_contract(
            &output.entries,
            Path::new("/proj"),
            since,
            until,
        );
    }

    #[test]
    fn test_extract_skips_rollout_outside_project_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("2026").join("03").join("01");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("rollout-2026-03-01T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-01T21:56:17Z","cwd":"/tmp/other","model_provider":"openai"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-03-01T21:58:00Z","type":"event_msg","payload":{{"type":"user_message","message":"should be skipped"}}}}"#
        )
        .unwrap();

        let output = extract_from_rollout(
            &path,
            Utc.with_ymd_and_hms(2026, 3, 1, 21, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 3, 1, 23, 0, 0).unwrap(),
            Path::new("/tmp/project"),
        )
        .unwrap();

        assert!(output.entries.is_empty());
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_extract_skips_rollout_when_cwd_resolves_outside_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let project_root = dir.path().join("proj");
        let other = dir.path().join("other");
        std::fs::create_dir_all(&project_root).unwrap();
        std::fs::create_dir_all(&other).unwrap();

        let nested = dir.path().join("2026").join("03").join("02");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("rollout-2026-03-02T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        let cwd = format!("{}/../other", project_root.display());
        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-02T21:56:17Z","cwd":"{cwd}","model_provider":"openai"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-03-02T21:58:00Z","type":"event_msg","payload":{{"type":"user_message","message":"should be skipped"}}}}"#
        )
        .unwrap();

        let output = extract_from_rollout(
            &path,
            Utc.with_ymd_and_hms(2026, 3, 2, 21, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 3, 2, 23, 0, 0).unwrap(),
            &project_root,
        )
        .unwrap();

        assert!(output.entries.is_empty());
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn test_extract_accepts_rollout_with_cwd_inside_root() {
        let dir = tempfile::TempDir::new().unwrap();
        let project_root = dir.path().join("proj");
        let nested_dir = project_root.join("nested");
        std::fs::create_dir_all(&nested_dir).unwrap();

        let nested = dir.path().join("2026").join("03").join("03");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("rollout-2026-03-03T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        let cwd = format!("{}/nested/..", project_root.display());
        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-03T21:56:17Z","cwd":"{cwd}","model_provider":"openai"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-03-03T21:58:00Z","type":"event_msg","payload":{{"type":"user_message","message":"should be captured"}}}}"#
        )
        .unwrap();

        let output = extract_from_rollout(
            &path,
            Utc.with_ymd_and_hms(2026, 3, 3, 21, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 3, 3, 23, 0, 0).unwrap(),
            &project_root,
        )
        .unwrap();

        assert_eq!(output.entries.len(), 1);
        assert_eq!(output.entries[0].prompt, "should be captured");
    }

    #[test]
    fn test_extract_warns_when_cwd_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("2026").join("03").join("01");
        std::fs::create_dir_all(&nested).unwrap();
        let path = nested.join("rollout-2026-03-01T13-56-17-testuuid.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();

        writeln!(
            f,
            r#"{{"type":"session_meta","payload":{{"id":"s1","timestamp":"2026-03-01T21:56:17Z","model_provider":"openai"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-03-01T21:58:00Z","type":"event_msg","payload":{{"type":"user_message","message":"should be skipped"}}}}"#
        )
        .unwrap();

        let output = extract_from_rollout(
            &path,
            Utc.with_ymd_and_hms(2026, 3, 1, 21, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 3, 1, 23, 0, 0).unwrap(),
            Path::new("/tmp/project"),
        )
        .unwrap();

        assert!(output.entries.is_empty());
        assert_eq!(output.warnings.len(), 1);
        assert!(output.warnings[0].contains("session cwd missing in"));
    }
}
