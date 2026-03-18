use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

use super::claude_code::ClaudeCodeExtractor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownToolKind {
    ClaudeCode,
    Codex,
    OpenCode,
    Cursor,
    GitHubCopilot,
    GeminiCli,
}

impl KnownToolKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex CLI / Desktop",
            Self::OpenCode => "OpenCode",
            Self::Cursor => "Cursor",
            Self::GitHubCopilot => "GitHub Copilot",
            Self::GeminiCli => "Gemini CLI / Antigravity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSupport {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPresenceStatus {
    ProjectRecent,
    ProjectStale,
    GlobalRecent,
    GlobalStale,
    InstalledNoLogs,
    NotDetected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDetection {
    pub kind: KnownToolKind,
    pub support: ToolSupport,
    pub status: ToolPresenceStatus,
    pub last_seen: Option<DateTime<Utc>>,
}

const MAX_SCAN_DEPTH: usize = 3;

pub fn detect_all_with_recency(
    project_root: &Path,
    now: DateTime<Utc>,
    recent_days: i64,
) -> Vec<ToolDetection> {
    detect_all_with_recency_inner(project_root, dirs::home_dir(), now, recent_days)
}

#[cfg(test)]
pub(crate) fn detect_all_with_recency_with_home(
    project_root: &Path,
    home: Option<PathBuf>,
    now: DateTime<Utc>,
    recent_days: i64,
) -> Vec<ToolDetection> {
    detect_all_with_recency_inner(project_root, home, now, recent_days)
}

fn detect_all_with_recency_inner(
    project_root: &Path,
    home: Option<PathBuf>,
    now: DateTime<Utc>,
    recent_days: i64,
) -> Vec<ToolDetection> {
    let mut detections = Vec::new();
    let threshold =
        now - chrono::Duration::try_days(recent_days).unwrap_or_else(chrono::Duration::zero);

    let categorize = |last_seen: Option<DateTime<Utc>>,
                      is_project: bool,
                      is_installed: bool|
     -> ToolPresenceStatus {
        match last_seen {
            Some(ts) if ts >= threshold => {
                if is_project {
                    ToolPresenceStatus::ProjectRecent
                } else {
                    ToolPresenceStatus::GlobalRecent
                }
            }
            Some(_) => {
                if is_project {
                    ToolPresenceStatus::ProjectStale
                } else {
                    ToolPresenceStatus::GlobalStale
                }
            }
            None => {
                if is_installed {
                    ToolPresenceStatus::InstalledNoLogs
                } else {
                    ToolPresenceStatus::NotDetected
                }
            }
        }
    };

    // 1. Claude Code
    let claude_home = home.as_ref().map(|h| h.join(".claude"));
    let claude_installed = claude_home.as_ref().is_some_and(|p| p.exists());
    let claude_proj = match home.as_ref() {
        Some(h) => claude_log_dir_for_home(project_root, h),
        None => ClaudeCodeExtractor::log_dir_for(project_root),
    };
    let claude_last_seen = claude_proj
        .as_ref()
        .and_then(|p| newest_mtime_limited(p, MAX_SCAN_DEPTH));
    detections.push(ToolDetection {
        kind: KnownToolKind::ClaudeCode,
        support: ToolSupport::Supported,
        status: categorize(claude_last_seen, claude_proj.is_some(), claude_installed),
        last_seen: claude_last_seen,
    });

    // 2. Codex
    let codex_home = if let Ok(h) = std::env::var("CODEX_HOME") {
        Some(std::path::PathBuf::from(h))
    } else {
        home.as_ref().map(|h| h.join(".codex"))
    };
    let codex_installed = codex_home.as_ref().is_some_and(|p| p.exists());
    let codex_sessions = codex_home.as_ref().and_then(|h| {
        let dir = h.join("sessions");
        if dir.exists() {
            Some(dir)
        } else {
            None
        }
    });
    let codex_last_seen = codex_sessions
        .as_ref()
        .and_then(|p| newest_mtime_limited(p, MAX_SCAN_DEPTH));
    detections.push(ToolDetection {
        kind: KnownToolKind::Codex,
        support: ToolSupport::Supported,
        status: categorize(codex_last_seen, false, codex_installed),
        last_seen: codex_last_seen,
    });

    // 3. OpenCode
    let opencode_dir = home
        .as_ref()
        .map(|h| h.join(".local").join("share").join("opencode"));
    let opencode_installed = opencode_dir.as_ref().is_some_and(|p| p.exists());
    let opencode_last_seen = opencode_dir
        .as_ref()
        .and_then(|p| newest_mtime_limited(p, MAX_SCAN_DEPTH));
    detections.push(ToolDetection {
        kind: KnownToolKind::OpenCode,
        support: ToolSupport::Unsupported,
        status: categorize(opencode_last_seen, false, opencode_installed),
        last_seen: opencode_last_seen,
    });

    // 4. Cursor
    let cursor_dir = if cfg!(target_os = "macos") {
        home.as_ref()
            .map(|h| h.join("Library").join("Application Support").join("Cursor"))
    } else {
        home.as_ref().map(|h| h.join(".config").join("Cursor"))
    };
    let cursor_installed = cursor_dir.as_ref().is_some_and(|p| p.exists());
    let cursor_last_seen = cursor_dir.as_ref().and_then(|p| {
        let user_data = p.join("User");
        if user_data.exists() {
            newest_mtime_limited(&user_data, MAX_SCAN_DEPTH)
        } else {
            newest_mtime_limited(p, MAX_SCAN_DEPTH)
        }
    });
    detections.push(ToolDetection {
        kind: KnownToolKind::Cursor,
        support: ToolSupport::Unsupported,
        status: categorize(cursor_last_seen, false, cursor_installed),
        last_seen: cursor_last_seen,
    });

    // 5. GitHub Copilot
    let copilot_dir = home
        .as_ref()
        .map(|h| h.join(".config").join("github-copilot"));
    let copilot_installed = copilot_dir.as_ref().is_some_and(|p| p.exists());
    let copilot_last_seen = copilot_dir
        .as_ref()
        .and_then(|p| newest_mtime_limited(p, MAX_SCAN_DEPTH));
    detections.push(ToolDetection {
        kind: KnownToolKind::GitHubCopilot,
        support: ToolSupport::Unsupported,
        status: categorize(copilot_last_seen, false, copilot_installed),
        last_seen: copilot_last_seen,
    });

    // 6. Gemini CLI
    let gemini_dir = home.as_ref().map(|h| h.join(".gemini"));
    let gemini_installed = gemini_dir.as_ref().is_some_and(|p| p.exists());
    let gemini_last_seen = gemini_dir
        .as_ref()
        .and_then(|p| newest_mtime_limited(p, MAX_SCAN_DEPTH));
    detections.push(ToolDetection {
        kind: KnownToolKind::GeminiCli,
        support: ToolSupport::Unsupported,
        status: categorize(gemini_last_seen, false, gemini_installed),
        last_seen: gemini_last_seen,
    });

    detections
}

fn newest_mtime_limited(dir: &Path, depth: usize) -> Option<DateTime<Utc>> {
    if depth == 0 || !dir.exists() || !dir.is_dir() {
        return None;
    }
    let mut newest: Option<DateTime<Utc>> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    if let Ok(mtime) = meta.modified() {
                        let dt: DateTime<Utc> = mtime.into();
                        newest = Some(match newest {
                            Some(n) => n.max(dt),
                            None => dt,
                        });
                    }
                } else if meta.is_dir() {
                    if let Some(dt) = newest_mtime_limited(&entry.path(), depth - 1) {
                        newest = Some(match newest {
                            Some(n) => n.max(dt),
                            None => dt,
                        });
                    }
                }
            }
        }
    }
    newest
}

fn claude_log_dir_for_home(project_root: &Path, home: &Path) -> Option<PathBuf> {
    let claude_projects = home.join(".claude").join("projects");
    let slug = project_root.to_string_lossy().replace('/', "-");
    let candidate = claude_projects.join(&slug);
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use std::fs;

    #[test]
    fn test_newest_mtime_depth_limit() {
        let dir = tempfile::TempDir::new().unwrap();
        let deep = dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        let f = deep.join("test.txt");
        fs::write(f, "hello").unwrap();

        assert!(newest_mtime_limited(dir.path(), 1).is_none());
        assert!(newest_mtime_limited(dir.path(), 4).is_some());
    }

    #[test]
    fn test_detect_all_with_recency_categorization() {
        let home = tempfile::TempDir::new().unwrap();
        let project = tempfile::TempDir::new().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();

        let claude_proj_slug = project.path().to_string_lossy().replace('/', "-");
        let claude_proj_dir = home
            .path()
            .join(".claude")
            .join("projects")
            .join(claude_proj_slug);
        fs::create_dir_all(&claude_proj_dir).unwrap();
        fs::write(claude_proj_dir.join("session.jsonl"), "log").unwrap();

        let codex_dir = home.path().join(".codex").join("sessions");
        fs::create_dir_all(&codex_dir).unwrap();
        let stale_file = codex_dir.join("stale.jsonl");
        fs::write(&stale_file, "stale").unwrap();
        let stale_time = now - Duration::try_days(10).unwrap();
        filetime::set_file_mtime(
            &stale_file,
            filetime::FileTime::from_unix_time(stale_time.timestamp(), 0),
        )
        .unwrap();

        let cursor_dir = if cfg!(target_os = "macos") {
            home.path()
                .join("Library")
                .join("Application Support")
                .join("Cursor")
        } else {
            home.path().join(".config").join("Cursor")
        };
        fs::create_dir_all(&cursor_dir).unwrap();
        fs::write(cursor_dir.join("recent.log"), "recent").unwrap();

        let detections = detect_all_with_recency_with_home(
            project.path(),
            Some(home.path().to_path_buf()),
            now,
            7,
        );

        let claude = detections
            .iter()
            .find(|d| d.kind == KnownToolKind::ClaudeCode)
            .unwrap();
        assert_eq!(claude.status, ToolPresenceStatus::ProjectRecent);
        assert_eq!(claude.support, ToolSupport::Supported);

        let codex = detections
            .iter()
            .find(|d| d.kind == KnownToolKind::Codex)
            .unwrap();
        assert_eq!(codex.status, ToolPresenceStatus::GlobalStale);

        let cursor = detections
            .iter()
            .find(|d| d.kind == KnownToolKind::Cursor)
            .unwrap();
        assert_eq!(cursor.status, ToolPresenceStatus::GlobalRecent);
        assert_eq!(cursor.support, ToolSupport::Unsupported);
    }

    #[test]
    fn test_detect_all_installed_no_logs() {
        let home = tempfile::TempDir::new().unwrap();
        let project = tempfile::TempDir::new().unwrap();
        let now = Utc::now();

        fs::create_dir_all(home.path().join(".claude")).unwrap();

        let detections = detect_all_with_recency_with_home(
            project.path(),
            Some(home.path().to_path_buf()),
            now,
            7,
        );
        let claude = detections
            .iter()
            .find(|d| d.kind == KnownToolKind::ClaudeCode)
            .unwrap();
        assert_eq!(claude.status, ToolPresenceStatus::InstalledNoLogs);
    }
}
