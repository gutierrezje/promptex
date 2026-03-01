//! `pmtx check` — report whether native log extraction is available.
//!
//! Intended to be called at the start of an agent skill. The **exit code**
//! is the primary contract:
//!
//! | Exit code | Meaning                                                  |
//! |-----------|----------------------------------------------------------|
//! | 0         | Native extractor found — prompts captured automatically  |
//! | 1         | No native support — agent should call `pmtx record`      |
//!
//! Example agent skill usage:
//! ```bash
//! if pmtx check; then
//!   : # native — nothing to do
//! else
//!   # after each significant action, call:
//!   # pmtx record --prompt "..." --files "..." --tool-calls "..." --outcome "..."
//! fi
//! ```

use std::env;

use anyhow::Result;

use crate::extractors::{self, ExtractorKind};
use crate::project_id;

pub fn execute() -> Result<()> {
    let cwd = env::current_dir()?;
    let pid = project_id::get_project_id(&cwd)?;
    let extractor = extractors::detect(&cwd, &pid);

    let primary = extractor.primary_kind();
    if is_native(primary) {
        eprintln!("✓ Native support: {}", primary.label());
        eprintln!("  Prompts are captured automatically — no setup required.");
        eprintln!("  Run `pmtx extract` when ready to generate PR output.");
    } else {
        eprintln!("⚠ No native support detected for your current tool.");
        eprintln!("  Call `pmtx record` after each significant prompt to capture it:\n");
        eprintln!("    pmtx record \\");
        eprintln!("      --prompt \"<your prompt>\" \\");
        eprintln!("      --files \"src/file1.rs,src/file2.rs\" \\");
        eprintln!("      --tool-calls \"Edit,Bash,Read\" \\");
        eprintln!("      --outcome \"<what was accomplished>\"");
        eprintln!();
        eprintln!("  Then run `pmtx extract` to generate PR-ready output.");
        std::process::exit(1);
    }

    Ok(())
}

/// Return true if `kind` corresponds to a natively supported tool.
///
/// `Manual` means the agent is not recognized — it must journal via
/// `pmtx record`. All other variants have dedicated log extractors.
pub fn is_native(kind: ExtractorKind) -> bool {
    kind != ExtractorKind::Manual
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_code_is_native() {
        assert!(is_native(ExtractorKind::ClaudeCode));
    }

    #[test]
    fn test_codex_is_native() {
        assert!(is_native(ExtractorKind::Codex));
    }

    #[test]
    fn test_manual_is_not_native() {
        assert!(!is_native(ExtractorKind::Manual));
    }
}
