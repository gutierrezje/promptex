//! Report whether prompt extraction appears available in the current repo.
//!
//! Exit code `0` means at least one extractor was detected. Exit code `1`
//! means none were detected.

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
            if kind.readiness() == "native" {
                eprintln!("✓ Native support: {}", kind.label());
                eprintln!("  Prompts are captured automatically — no setup required.");
            } else {
                eprintln!("⚠ WIP support detected: {}", kind.label());
                eprintln!("  Extraction exists, but treat results as provisional.");
            }
            eprintln!("  Run `pmtx extract` when ready to generate PR output.");
        }
        None => {
            eprintln!("⚠ No native support detected for your current tool.");
            eprintln!(
                "  pmtx currently supports Claude Code natively; Codex support is still WIP."
            );
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
