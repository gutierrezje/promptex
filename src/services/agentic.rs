use anyhow::Result;

/// Check if Claude Code CLI is available
pub fn is_claude_available() -> bool {
    // TODO: Implement Claude CLI detection
    todo!("Check if Claude CLI is available")
}

/// Invoke Claude Code to synthesize RULES.md
pub async fn synthesize_rules(
    repo_path: &str,
    contributing_content: Option<&str>,
    ci_config_content: Option<&str>,
) -> Result<String> {
    // TODO: Implement Claude Code invocation
    todo!("Synthesize RULES.md via Claude Code")
}
