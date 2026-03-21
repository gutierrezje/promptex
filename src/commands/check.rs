//! Report whether prompt extraction appears available in the current repo.
//!
//! Exit code `0` means at least one supported tool was detected. Exit code `1`
//! means no supported tools were detected.

use std::env;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::extractors::{detect_all_with_recency, ToolDetection, ToolPresenceStatus, ToolSupport};

#[derive(Debug, Default)]
struct CheckOutcome {
    lines: Vec<String>,
    supported_capable: bool,
}

fn build_check_outcome(detections: &[ToolDetection], now: DateTime<Utc>) -> CheckOutcome {
    let mut supported_capable = false;
    let mut project_recent_supported = false;
    let mut any_recent_unsupported = false;

    let mut supported_lines = Vec::new();
    let mut unsupported_lines = Vec::new();

    for d in detections {
        let is_recent = matches!(
            d.status,
            ToolPresenceStatus::ProjectRecent | ToolPresenceStatus::GlobalRecent
        );
        let is_capable = matches!(
            d.status,
            ToolPresenceStatus::ProjectRecent
                | ToolPresenceStatus::ProjectStale
                | ToolPresenceStatus::GlobalRecent
                | ToolPresenceStatus::GlobalStale
                | ToolPresenceStatus::InstalledNoLogs
        );

        if d.support == ToolSupport::Supported {
            if is_capable {
                supported_capable = true;
            }
            if d.status == ToolPresenceStatus::ProjectRecent {
                project_recent_supported = true;
            }
        } else if d.support == ToolSupport::Unsupported && is_recent {
            any_recent_unsupported = true;
        }

        let label = d.kind.label();
        let status_text = match d.status {
            ToolPresenceStatus::ProjectRecent | ToolPresenceStatus::GlobalRecent => {
                if let Some(ts) = d.last_seen {
                    let days = (now - ts).num_days();
                    if days == 0 {
                        "active today".to_string()
                    } else {
                        format!("active {}d ago", days)
                    }
                } else {
                    "recently active".to_string()
                }
            }
            ToolPresenceStatus::ProjectStale | ToolPresenceStatus::GlobalStale => {
                if let Some(ts) = d.last_seen {
                    let days = (now - ts).num_days();
                    format!("last seen {}d ago", days)
                } else {
                    "stale logs found".to_string()
                }
            }
            ToolPresenceStatus::InstalledNoLogs => "installed, no logs yet".to_string(),
            ToolPresenceStatus::NotDetected => continue,
        };

        if d.support == ToolSupport::Supported {
            supported_lines.push(format!("* Supported: {} ({})", label, status_text));
        } else {
            unsupported_lines.push(format!(
                "Warning: {} detected ({}) but not yet supported by pmtx.",
                label, status_text
            ));
        }
    }

    let mut lines = Vec::new();

    if supported_capable {
        lines.extend(supported_lines);
    } else {
        lines.push("Warning: No supported tool detected in your current environment.".to_string());
    }

    if !unsupported_lines.is_empty() {
        if supported_capable {
            lines.push(String::new());
        }
        lines.extend(unsupported_lines);
    }

    let likely_current_unsupported = any_recent_unsupported && !project_recent_supported;
    if likely_current_unsupported {
        lines.push(String::new());
        lines.push(
            "Warning: Your most recent AI activity appears to be in unsupported tools; pmtx extract may miss current sessions.".to_string(),
        );
    }

    if supported_capable {
        lines.push(String::new());
        lines.push("  Run `pmtx extract` when ready to generate PR output.".to_string());
    }

    CheckOutcome {
        lines,
        supported_capable,
    }
}

/// Run the `pmtx check` command.
pub fn execute() -> Result<()> {
    let cwd = env::current_dir()?;
    let now = Utc::now();

    // We check recency over a default 7-day window.
    let detections = detect_all_with_recency(&cwd, now, 7);

    let outcome = build_check_outcome(&detections, now);
    for line in outcome.lines {
        eprintln!("{}", line);
    }

    if outcome.supported_capable {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use crate::extractors::detection::KnownToolKind;
    use crate::extractors::{ToolDetection, ToolPresenceStatus, ToolSupport};

    use super::build_check_outcome;

    #[test]
    fn test_claude_code_kind_label() {
        assert_eq!(KnownToolKind::ClaudeCode.label(), "Claude Code");
    }

    #[test]
    fn test_codex_kind_label() {
        assert_eq!(KnownToolKind::Codex.label(), "Codex CLI / Desktop");
    }

    #[test]
    fn warns_when_recent_unsupported_but_supported_is_stale() {
        let now = Utc.with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let detections = vec![
            ToolDetection {
                kind: KnownToolKind::ClaudeCode,
                support: ToolSupport::Supported,
                status: ToolPresenceStatus::ProjectStale,
                last_seen: Some(now - Duration::days(21)),
            },
            ToolDetection {
                kind: KnownToolKind::OpenCode,
                support: ToolSupport::Unsupported,
                status: ToolPresenceStatus::GlobalRecent,
                last_seen: Some(now - Duration::days(1)),
            },
        ];

        let outcome = build_check_outcome(&detections, now);
        assert!(outcome.supported_capable);
        assert!(outcome.lines.iter().any(|line| {
            line.contains("most recent AI activity appears to be in unsupported tools")
        }));
    }

    #[test]
    fn does_not_warn_when_project_supported_is_recent() {
        let now = Utc.with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let detections = vec![
            ToolDetection {
                kind: KnownToolKind::ClaudeCode,
                support: ToolSupport::Supported,
                status: ToolPresenceStatus::ProjectRecent,
                last_seen: Some(now - Duration::hours(4)),
            },
            ToolDetection {
                kind: KnownToolKind::OpenCode,
                support: ToolSupport::Unsupported,
                status: ToolPresenceStatus::GlobalRecent,
                last_seen: Some(now - Duration::days(1)),
            },
        ];

        let outcome = build_check_outcome(&detections, now);
        assert!(outcome.supported_capable);
        assert!(!outcome.lines.iter().any(|line| {
            line.contains("most recent AI activity appears to be in unsupported tools")
        }));
    }

    #[test]
    fn reports_no_supported_tools_when_only_unsupported_recent() {
        let now = Utc.with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let detections = vec![ToolDetection {
            kind: KnownToolKind::Cursor,
            support: ToolSupport::Unsupported,
            status: ToolPresenceStatus::GlobalRecent,
            last_seen: Some(now - Duration::days(1)),
        }];

        let outcome = build_check_outcome(&detections, now);
        assert!(!outcome.supported_capable);
        assert!(outcome
            .lines
            .iter()
            .any(|line| line.contains("No supported tool detected")));
    }

    #[test]
    fn treats_installed_supported_with_no_logs_as_capable() {
        let now = Utc.with_ymd_and_hms(2026, 3, 18, 12, 0, 0).unwrap();
        let detections = vec![ToolDetection {
            kind: KnownToolKind::Codex,
            support: ToolSupport::Supported,
            status: ToolPresenceStatus::InstalledNoLogs,
            last_seen: None,
        }];

        let outcome = build_check_outcome(&detections, now);
        assert!(outcome.supported_capable);
        assert!(outcome
            .lines
            .iter()
            .any(|line| line.contains("installed, no logs yet")));
    }
}
