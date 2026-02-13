use anyhow::{Context, Result, anyhow, bail};
use std::process::Command;

/// Check if OpenCode CLI is available
pub fn is_opencode_available() -> bool {
    Command::new("opencode")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Invoke OpenCode to synthesize RULES.md
pub async fn synthesize_rules(
    repo_path: &str,
    contributing_content: Option<&str>,
    ci_config_content: Option<&str>,
) -> Result<String> {
    if !is_opencode_available() {
        bail!("OpenCode CLI not found in PATH");
    }

    let repo_path = repo_path.to_string();
    let prompt = build_rules_prompt(contributing_content, ci_config_content);

    tokio::task::spawn_blocking(move || invoke_opencode(&repo_path, &prompt))
        .await
        .context("OpenCode task failed")?
}

fn build_rules_prompt(contributing_content: Option<&str>, ci_config_content: Option<&str>) -> String {
    let contributing = contributing_content.unwrap_or("No CONTRIBUTING.md content found.");
    let ci_config = ci_config_content.unwrap_or("No CI config content found.");

    format!(
        "You are synthesizing contributor rules for an open source repository.\n\
         Use only the provided sources. Do not invent facts.\n\
         Output markdown with sections: Commit Convention, Testing, Style, Review Process, Don'ts.\n\n\
         [CONTRIBUTING.md]\n{}\n\n\
         [CI CONFIG]\n{}\n",
        contributing, ci_config
    )
}

fn invoke_opencode(repo_path: &str, prompt: &str) -> Result<String> {
    // Try explicit prompt mode first.
    let explicit = Command::new("opencode")
        .current_dir(repo_path)
        .arg("prompt")
        .arg(prompt)
        .output();

    let output = match explicit {
        Ok(out) if out.status.success() => out,
        Ok(_) | Err(_) => {
            // Fallback to plain positional prompt style.
            Command::new("opencode")
                .current_dir(repo_path)
                .arg(prompt)
                .output()
                .context("Failed to invoke OpenCode CLI")?
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("OpenCode failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        bail!("OpenCode returned empty output");
    }

    Ok(stdout)
}
