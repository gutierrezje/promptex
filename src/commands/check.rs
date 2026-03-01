//! `pmtx check` — report whether native log extraction is available.
//!
//! Intended to be called at the start of an agent skill. The **exit code**
//! is the primary contract:
//!
//! | Exit code | Meaning                                              |
//! |-----------|------------------------------------------------------|
//! | 0         | Native extractor found — prompts captured automatically |
//! | 1         | Tool not supported — pmtx cannot extract prompts    |

use std::env;

use anyhow::Result;

use crate::extractors;
use crate::project_id;

pub fn execute() -> Result<()> {
    let cwd = env::current_dir()?;
    let pid = project_id::get_project_id(&cwd)?;
    let extractor = extractors::detect(&cwd, &pid);

    match extractor.primary_kind() {
        Some(kind) => {
            eprintln!("✓ Native support: {}", kind.label());
            eprintln!("  Prompts are captured automatically — no setup required.");
            eprintln!("  Run `pmtx extract` when ready to generate PR output.");
        }
        None => {
            eprintln!("⚠ No native support detected for your current tool.");
            eprintln!("  pmtx can only extract from supported tools (Claude Code, Codex).");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::extractors::ExtractorKind;

    #[test]
    fn test_claude_code_kind_label() {
        assert_eq!(ExtractorKind::ClaudeCode.label(), "Claude Code");
    }

    #[test]
    fn test_codex_kind_label() {
        assert_eq!(ExtractorKind::Codex.label(), "Codex CLI / Desktop");
    }
}
