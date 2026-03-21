//! Extract prompts from Cursor agent transcript JSONL logs.
//!
//! Cursor stores per-workspace transcripts under:
//! `~/.cursor/projects/{workspace-slug}/agent-transcripts/`.
//! This extractor scans top-level transcript files (excluding `subagents/`),
//! captures user prompts, and infers tool/file activity from following
//! assistant messages.

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use dirs::home_dir;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::traits::PromptExtractor;
use super::ExtractorOutput;
use crate::analysis::git;
use crate::prompt::PromptEntry;

pub struct CursorExtractor {
    transcripts_dir: PathBuf,
    project_root: PathBuf,
}

impl CursorExtractor {
    pub fn new(transcripts_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            transcripts_dir,
            project_root,
        }
    }

    pub fn transcripts_dir_for(project_root: &Path) -> Option<PathBuf> {
        let home = home_dir()?;
        let raw_slug = project_root.to_string_lossy().replace('/', "-");
        let trimmed_slug = raw_slug.trim_start_matches('-');
        let base = home.join(".cursor").join("projects");

        for slug in [raw_slug.as_str(), trimmed_slug] {
            let dir = base.join(slug).join("agent-transcripts");
            if dir.exists() {
                return Some(dir);
            }
        }

        None
    }
}

impl PromptExtractor for CursorExtractor {
    fn is_available(project_root: &Path) -> bool {
        Self::transcripts_dir_for(project_root).is_some()
    }

    fn extract(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<ExtractorOutput> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        let branch = git::current_branch().unwrap_or_else(|_| "unknown".to_string());
        let (model_timeline, mut model_warnings) = load_cursor_model_timeline();
        warnings.append(&mut model_warnings);

        let mut transcript_files = collect_jsonl_files(&self.transcripts_dir);
        transcript_files.sort();

        for transcript in transcript_files {
            match extract_from_transcript(
                &transcript,
                since,
                until,
                &self.project_root,
                &branch,
                &model_timeline,
            ) {
                Ok(mut out) => {
                    entries.append(&mut out.entries);
                    warnings.append(&mut out.warnings);
                }
                Err(err) => warnings.push(format!(
                    "failed to extract transcript from {}: {err}",
                    transcript.display()
                )),
            }
        }

        entries.sort_by_key(|e| e.timestamp);
        Ok(ExtractorOutput { entries, warnings })
    }
}

struct TranscriptExtractOutput {
    entries: Vec<PromptEntry>,
    warnings: Vec<String>,
}

fn collect_jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "subagents") {
                continue;
            }
            files.extend(collect_jsonl_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }
    files
}

fn extract_from_transcript(
    path: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    project_root: &Path,
    branch: &str,
    model_timeline: &HashMap<String, Vec<ModelSelection>>,
) -> Result<TranscriptExtractOutput> {
    let file = File::open(path).context("Failed to open cursor transcript")?;
    let reader = BufReader::new(file);

    let mut messages = Vec::new();
    let mut warnings = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(v) => messages.push(v),
            Err(err) => warnings.push(format!(
                "skipped invalid JSON line at {}:{}: {err}",
                path.display(),
                idx + 1
            )),
        }
    }

    let file_ts = file_timestamp(path);
    let mut fallback_counter: i64 = 0;
    let mut entries = Vec::new();
    let mut i = 0;
    let composer_id = composer_id_from_transcript_path(path);
    while i < messages.len() {
        if is_user_message(&messages[i]) {
            if let Some(prompt) = extract_user_text(&messages[i]) {
                if let Some(ts) = extract_message_timestamp(&messages[i])
                    .or_else(|| message_level_timestamp(&messages[i]))
                    .or_else(|| {
                        file_ts.map(|base| {
                            let ts = base + chrono::Duration::minutes(fallback_counter);
                            fallback_counter += 1;
                            ts
                        })
                    })
                {
                    if ts >= since && ts <= until {
                        let (mut tool_calls, files_touched, model) =
                            collect_assistant_context(&messages, i + 1);
                        if detect_skill_usage(&files_touched) {
                            push_unique(&mut tool_calls, "Skill");
                        }
                        let files_touched = normalize_files_touched(files_touched, project_root);

                        let resolved_model = model.or_else(|| {
                            composer_id
                                .as_deref()
                                .and_then(|id| resolve_model_for_prompt(model_timeline, id, ts))
                        });
                        let mut entry = PromptEntry::new(
                            "unknown".to_string(),
                            String::new(),
                            prompt,
                            files_touched,
                            tool_calls,
                            "cursor".to_string(),
                            resolved_model,
                        );
                        entry.branch = branch.to_string();
                        entry.timestamp = ts;
                        entries.push(entry);
                    }
                }
            }
        }
        i += 1;
    }

    Ok(TranscriptExtractOutput { entries, warnings })
}

#[derive(Clone)]
struct ModelSelection {
    ts: DateTime<Utc>,
    model: String,
}

fn composer_id_from_transcript_path(path: &Path) -> Option<String> {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

fn resolve_model_for_prompt(
    timeline: &HashMap<String, Vec<ModelSelection>>,
    composer_id: &str,
    prompt_ts: DateTime<Utc>,
) -> Option<String> {
    let events = timeline.get(composer_id)?;
    for event in events.iter().rev() {
        if event.ts <= prompt_ts {
            return Some(event.model.clone());
        }
    }
    events.last().map(|e| e.model.clone())
}

fn load_cursor_model_timeline() -> (HashMap<String, Vec<ModelSelection>>, Vec<String>) {
    let mut map: HashMap<String, Vec<ModelSelection>> = HashMap::new();
    let mut warnings = Vec::new();
    let Some(home) = home_dir() else {
        return (map, warnings);
    };
    let logs_root = home
        .join("Library")
        .join("Application Support")
        .join("Cursor")
        .join("logs");
    let renderer_logs = collect_renderer_logs(&logs_root);
    for path in renderer_logs {
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(err) => {
                warnings.push(format!(
                    "failed to read renderer log {}: {err}",
                    path.display()
                ));
                continue;
            }
        };
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Some((ts, composer_id, model)) = parse_build_requested_model_line(&line) {
                map.entry(composer_id)
                    .or_default()
                    .push(ModelSelection { ts, model });
            }
        }
    }
    for events in map.values_mut() {
        events.sort_by_key(|e| e.ts);
    }
    (map, warnings)
}

fn collect_renderer_logs(root: &Path) -> Vec<PathBuf> {
    let mut logs = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return logs;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            logs.extend(collect_renderer_logs(&path));
        } else if path.file_name().and_then(|n| n.to_str()) == Some("renderer.log") {
            logs.push(path);
        }
    }
    logs
}

fn parse_build_requested_model_line(line: &str) -> Option<(DateTime<Utc>, String, String)> {
    if !line.contains("[buildRequestedModel]") {
        return None;
    }
    let ts = parse_renderer_timestamp(line)?;
    let composer_id = parse_kv_token(line, "composerId=")?;
    let model = parse_kv_token(line, "catalogModelId=")?;
    if model.is_empty() {
        return None;
    }
    Some((ts, composer_id, model))
}

fn parse_renderer_timestamp(line: &str) -> Option<DateTime<Utc>> {
    let stamp = line.get(..23)?;
    let naive = NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M:%S%.3f").ok()?;
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.with_timezone(&Utc))
}

fn parse_kv_token(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let tail = &line[start..];
    let end = tail.find(char::is_whitespace).unwrap_or(tail.len());
    let value = tail[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn file_timestamp(path: &Path) -> Option<DateTime<Utc>> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    Some(modified.into())
}

fn is_user_message(message: &Value) -> bool {
    message.get("role").and_then(|v| v.as_str()) == Some("user")
}

fn extract_user_text(message: &Value) -> Option<String> {
    let content = message
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())?;

    let text = content
        .iter()
        .filter(|part| part.get("type").and_then(|v| v.as_str()) == Some("text"))
        .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    normalize_user_text(&text)
}

fn extract_message_timestamp(message: &Value) -> Option<DateTime<Utc>> {
    for key in ["timestamp", "created_at", "createdAt"] {
        if let Some(ts) = message
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
        {
            return Some(ts);
        }
    }
    None
}

fn message_level_timestamp(message: &Value) -> Option<DateTime<Utc>> {
    let msg = message.get("message")?;
    for key in ["timestamp", "created_at", "createdAt"] {
        if let Some(ts) = msg
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
        {
            return Some(ts);
        }
    }
    None
}

fn normalize_user_text(raw: &str) -> Option<String> {
    let mut text = raw.trim().to_string();
    while text.contains("\\\\n") {
        text = text.replace("\\\\n", "\n");
    }
    text = text
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\t", "\t");

    if let (Some(start), Some(end)) = (text.find("<user_query>"), text.find("</user_query>")) {
        if end > start {
            let inner = &text[start + "<user_query>".len()..end];
            text = inner.trim().to_string();
        }
    }

    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn collect_assistant_context(
    messages: &[Value],
    start: usize,
) -> (Vec<String>, Vec<String>, Option<String>) {
    let mut tool_calls = Vec::new();
    let mut files_touched = Vec::new();
    let mut model: Option<String> = None;

    for msg in &messages[start..] {
        if is_user_message(msg) {
            break;
        }
        if msg.get("role").and_then(|v| v.as_str()) != Some("assistant") {
            continue;
        }
        if model.is_none() {
            model = extract_model_from_message(msg);
        }

        let Some(content) = msg
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
        else {
            continue;
        };

        for part in content {
            if part.get("type").and_then(|v| v.as_str()) != Some("tool_use") {
                continue;
            }
            let Some(name) = part.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let input = part.get("input");

            let command_categories = push_tool_call(&mut tool_calls, name, input);
            let is_shell = name.eq_ignore_ascii_case("shell");
            let captures_write_paths = if is_shell {
                command_categories.iter().any(|c| c == "Write")
            } else {
                is_write_tool_name(name)
            };

            if captures_write_paths {
                if let Some(input) = input {
                    collect_paths_from_named_fields(input, &mut files_touched);
                    if let Some(cmd) = extract_command_from_input(input) {
                        extract_paths_from_command(&cmd, &mut files_touched);
                    }
                    if name.eq_ignore_ascii_case("applypatch")
                        || name.eq_ignore_ascii_case("apply_patch")
                    {
                        if let Some(patch) = input.as_str() {
                            extract_paths_from_apply_patch(patch, &mut files_touched);
                        } else if let Some(patch) = input.get("input").and_then(|v| v.as_str()) {
                            extract_paths_from_apply_patch(patch, &mut files_touched);
                        }
                    }
                }
            }
            if model.is_none() {
                model = input.and_then(extract_model_from_value);
            }
        }
    }

    (tool_calls, files_touched, model)
}

fn extract_command_from_input(input: &Value) -> Option<String> {
    if let Some(command) = input.get("command") {
        match command {
            Value::String(s) => return Some(s.to_string()),
            Value::Array(parts) => {
                let joined = parts
                    .iter()
                    .filter_map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !joined.is_empty() {
                    return Some(joined);
                }
            }
            _ => {}
        }
    }
    if let Some(cmd) = input.get("cmd").and_then(|v| v.as_str()) {
        return Some(cmd.to_string());
    }
    None
}

fn normalize_tool_name(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    match n.as_str() {
        "readfile" | "read_file" | "read" => Some("Read"),
        "glob" | "rg" | "semanticsearch" | "subagent" => Some("Explore"),
        "applypatch" | "apply_patch" | "delete" | "editnotebook" => Some("Write"),
        "todowrite" => None,
        _ => None,
    }
}

fn push_tool_call(tool_calls: &mut Vec<String>, name: &str, input: Option<&Value>) -> Vec<String> {
    if name.eq_ignore_ascii_case("shell") {
        let cmd = input.and_then(extract_command_from_input);
        return push_command_tool_calls(tool_calls, cmd.as_deref());
    }

    if let Some(normalized) = normalize_tool_name(name) {
        push_unique(tool_calls, normalized);
    }
    Vec::new()
}

fn push_command_tool_calls(tool_calls: &mut Vec<String>, cmd: Option<&str>) -> Vec<String> {
    if let Some(cmd) = cmd {
        let inferred = infer_command_categories(cmd);
        if !inferred.is_empty() {
            for category in &inferred {
                push_unique(tool_calls, category);
            }
            return inferred;
        }
    }
    push_unique(tool_calls, "Bash");
    vec!["Bash".to_string()]
}

fn is_write_tool_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "applypatch"
            | "apply_patch"
            | "delete"
            | "editnotebook"
            | "writefile"
            | "write_file"
            | "editfile"
            | "createfile"
            | "create_file"
    )
}

fn extract_model_from_message(message: &Value) -> Option<String> {
    let msg = message.get("message");
    for key in ["model", "model_name", "model_slug", "model_id"] {
        if let Some(v) = message.get(key).and_then(|v| v.as_str()) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        if let Some(v) = msg.and_then(|m| m.get(key)).and_then(|v| v.as_str()) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn extract_model_from_value(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in ["model", "model_name", "model_slug", "model_id"] {
                if let Some(v) = map.get(key).and_then(|v| v.as_str()) {
                    let t = v.trim();
                    if !t.is_empty() {
                        return Some(t.to_string());
                    }
                }
            }
            for child in map.values() {
                if let Some(found) = extract_model_from_value(child) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => arr.iter().find_map(extract_model_from_value),
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
    for word in segment.split_whitespace() {
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

fn collect_paths_from_named_fields(value: &Value, files_touched: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                match key.as_str() {
                    "path" | "file_path" | "filepath" | "file" | "filename" | "target_notebook" => {
                        match child {
                            Value::String(s) => push_path(files_touched, s),
                            Value::Array(arr) => {
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        push_path(files_touched, s);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
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
    for token in cmd.split_whitespace() {
        let token = token.trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        });
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
    if token.is_empty() || token == "/" || token.starts_with('-') {
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
    if token.starts_with("~/")
        || token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with('/')
        || token.contains('/')
    {
        return true;
    }

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
                if rel.as_os_str().is_empty() {
                    continue;
                }
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
            let canonical_root = canonicalize_dir(&project_root);
            if let Ok(rel) = canonical_joined.strip_prefix(&canonical_root) {
                if rel.as_os_str().is_empty() {
                    continue;
                }
                push_unique(&mut normalized, &rel.to_string_lossy());
            }
        } else {
            let joined_normalized = canonicalize_dir(&joined);
            if let Ok(rel) = joined_normalized.strip_prefix(&project_root) {
                if rel.as_os_str().is_empty() {
                    continue;
                }
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

fn canonicalize_dir(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn detect_skill_usage(paths: &[String]) -> bool {
    paths.iter().any(|path| {
        path.starts_with("skills/")
            || path.contains("/skills/")
            || path.contains("/.agents/skills/")
            || path.contains("/.codex/skills/")
    })
}

fn push_unique(vec: &mut Vec<String>, item: &str) {
    let s = item.to_string();
    if !vec.contains(&s) {
        vec.push(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::test_contract::assert_entries_contract;
    use chrono::TimeZone;
    use std::io::Write;

    #[test]
    fn test_extract_from_transcript_happy_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let transcript = dir.path().join("chat.jsonl");
        let mut f = std::fs::File::create(&transcript).unwrap();

        writeln!(
            f,
            r#"{{"role":"user","timestamp":"2026-03-19T11:00:00Z","message":{{"content":[{{"type":"text","text":"implement cursor extractor"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"role":"assistant","message":{{"content":[{{"type":"tool_use","name":"ReadFile","input":{{"path":"src/extractors/mod.rs"}}}},{{"type":"tool_use","name":"ApplyPatch","input":{{"input":"*** Begin Patch\n*** Update File: src/extractors/mod.rs\n*** End Patch\n"}}}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"role":"user","timestamp":"2026-03-19T11:05:00Z","message":{{"content":[{{"type":"text","text":"run tests"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"role":"assistant","message":{{"content":[{{"type":"tool_use","name":"Shell","input":{{"command":"cargo test"}}}}]}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2026, 3, 19, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 19, 12, 0, 0).unwrap();
        let out = extract_from_transcript(
            &transcript,
            since,
            until,
            Path::new("/proj"),
            "main",
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0].prompt, "implement cursor extractor");
        assert_eq!(out.entries[0].tool_calls, vec!["Read", "Write"]);
        assert_eq!(out.entries[0].files_touched, vec!["src/extractors/mod.rs"]);
        assert_eq!(out.entries[0].tool, "cursor");
        assert_eq!(out.entries[1].tool_calls, vec!["Bash"]);
    }

    #[test]
    fn test_time_filtering() {
        let dir = tempfile::TempDir::new().unwrap();
        let transcript = dir.path().join("chat.jsonl");
        let mut f = std::fs::File::create(&transcript).unwrap();
        writeln!(
            f,
            r#"{{"role":"user","timestamp":"2026-03-19T08:00:00Z","message":{{"content":[{{"type":"text","text":"too early"}}]}}}}"#
        )
        .unwrap();
        let since = Utc.with_ymd_and_hms(2026, 3, 19, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 19, 12, 0, 0).unwrap();
        let out = extract_from_transcript(
            &transcript,
            since,
            until,
            Path::new("/proj"),
            "main",
            &HashMap::new(),
        )
        .unwrap();
        assert!(out.entries.is_empty());
    }

    #[test]
    fn test_path_normalization_and_dedup() {
        let files = vec![
            "/proj/src/lib.rs".to_string(),
            "/proj/src/lib.rs".to_string(),
            "../outside.rs".to_string(),
            "~/secret.txt".to_string(),
            "src/main.rs".to_string(),
        ];
        let normalized = normalize_files_touched(files, Path::new("/proj"));
        assert_eq!(normalized, vec!["src/lib.rs", "src/main.rs"]);
    }

    #[test]
    fn test_collect_jsonl_skips_subagents() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path().join("agent-transcripts");
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::create_dir_all(root.join("subagents")).unwrap();
        std::fs::write(root.join("a").join("keep.jsonl"), "{}\n").unwrap();
        std::fs::write(root.join("subagents").join("skip.jsonl"), "{}\n").unwrap();

        let files = collect_jsonl_files(&root);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("keep.jsonl"));
    }

    #[test]
    fn test_extract_reports_bad_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let transcript = dir.path().join("chat.jsonl");
        let mut f = std::fs::File::create(&transcript).unwrap();
        writeln!(f, "{{not-json").unwrap();
        writeln!(
            f,
            r#"{{"role":"user","timestamp":"2026-03-19T11:00:00Z","message":{{"content":[{{"type":"text","text":"valid prompt"}}]}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2026, 3, 19, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 19, 12, 0, 0).unwrap();
        let out = extract_from_transcript(
            &transcript,
            since,
            until,
            Path::new("/proj"),
            "main",
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("skipped invalid JSON line at"));
    }

    #[test]
    fn test_entries_contract_compliance() {
        let dir = tempfile::TempDir::new().unwrap();
        let transcript = dir.path().join("chat.jsonl");
        let mut f = std::fs::File::create(&transcript).unwrap();
        writeln!(
            f,
            r#"{{"role":"user","timestamp":"2026-03-19T11:00:00Z","message":{{"content":[{{"type":"text","text":"do work"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"role":"assistant","message":{{"content":[{{"type":"tool_use","name":"ReadFile","input":{{"path":"src/lib.rs"}}}}]}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2026, 3, 19, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 3, 19, 12, 0, 0).unwrap();
        let out = extract_from_transcript(
            &transcript,
            since,
            until,
            Path::new("/proj"),
            "main",
            &HashMap::new(),
        )
        .unwrap();
        assert_entries_contract(&out.entries, Path::new("/proj"), since, until);
    }

    #[test]
    fn test_normalize_user_query_wrapper_and_newlines() {
        let raw = "<user_query>\\nline one\\nline two\\n</user_query>";
        assert_eq!(
            normalize_user_text(raw).as_deref(),
            Some("line one\nline two")
        );
    }

    #[test]
    fn test_normalize_user_query_double_escaped_newlines() {
        let raw = "<user_query>\\\\nline one\\\\nline two\\\\n</user_query>";
        assert_eq!(
            normalize_user_text(raw).as_deref(),
            Some("line one\nline two")
        );
    }

    #[test]
    fn test_file_timestamp_fallback_increments_per_prompt() {
        let dir = tempfile::TempDir::new().unwrap();
        let transcript = dir.path().join("chat.jsonl");
        let mut f = std::fs::File::create(&transcript).unwrap();
        writeln!(
            f,
            r#"{{"role":"user","message":{{"content":[{{"type":"text","text":"first"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"role":"user","message":{{"content":[{{"type":"text","text":"second"}}]}}}}"#
        )
        .unwrap();

        let since = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap();
        let out = extract_from_transcript(
            &transcript,
            since,
            until,
            Path::new("/proj"),
            "main",
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(out.entries.len(), 2);
        assert!(out.entries[1].timestamp > out.entries[0].timestamp);
    }

    #[test]
    fn test_normalize_files_touched_drops_project_root_path() {
        let files = vec!["/proj".to_string(), "/proj/src/lib.rs".to_string()];
        let normalized = normalize_files_touched(files, Path::new("/proj"));
        assert_eq!(normalized, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_is_path_like_token_rejects_code_like_dotted_token() {
        assert!(!is_path_like_token("data.get('entries"));
        assert!(!is_path_like_token(
            "json.dump({'version':1,'decisions':dec},f"
        ));
    }

    #[test]
    fn test_parse_build_requested_model_line() {
        let line = "2026-03-19 17:59:55.412 [info] [buildRequestedModel] composerId=abc-123 catalogModelId=composer-2 idSource=selectedModels[0]";
        let parsed = parse_build_requested_model_line(line).unwrap();
        assert_eq!(parsed.1, "abc-123");
        assert_eq!(parsed.2, "composer-2");
    }

    #[test]
    fn test_resolve_model_for_prompt_uses_latest_before_timestamp() {
        let ts1 = Utc.with_ymd_and_hms(2026, 3, 19, 10, 0, 0).unwrap();
        let ts2 = Utc.with_ymd_and_hms(2026, 3, 19, 11, 0, 0).unwrap();
        let mut timeline = HashMap::new();
        timeline.insert(
            "composer-id".to_string(),
            vec![
                ModelSelection {
                    ts: ts1,
                    model: "default".to_string(),
                },
                ModelSelection {
                    ts: ts2,
                    model: "composer-2".to_string(),
                },
            ],
        );
        let model = resolve_model_for_prompt(
            &timeline,
            "composer-id",
            Utc.with_ymd_and_hms(2026, 3, 19, 11, 30, 0).unwrap(),
        );
        assert_eq!(model.as_deref(), Some("composer-2"));
    }
}
