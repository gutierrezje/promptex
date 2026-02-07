use anyhow::{Context, Result, anyhow};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueUrl {
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub author: String,
    pub created_at: String,
    pub labels: Vec<String>,
    pub milestone: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub is_maintainer: bool,
}

impl IssueUrl {
    /// Parse a GitHub issue URL into owner/repo/issue_number
    /// Supports formats:
    /// - https://github.com/owner/repo/issues/123
    /// - github.com/owner/repo/issues/123
    pub fn parse(url: &str) -> Result<Self> {
        let url = if url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        };
        let url_str = Url::parse(&url).context("Failed to parse URL")?;
        if url_str.host_str() != Some("github.com") && url_str.host_str() != Some("www.github.com") {
            return Err(anyhow!("URL must be from github.com"));
        }
        let mut segments = url_str.path_segments().context("URL must have path segments")?;
        let owner = segments.next().context("URL must have owner segment")?;
        let repo = segments.next().context("URL must have repo segment")?;
        let kind = segments.next().context("URL must have 'issues' segment")?;
        if kind != "issues" {
            return Err(anyhow!("URL must contain 'issues' segment"));
        }
        let issue_number_str = segments.next().context("URL must have issue number segment")?;
        let issue_number = issue_number_str.parse::<u64>().context("Issue number must be a valid integer")?;

        if segments.next().is_some() {
            return Err(anyhow!("URL has too many segments"));
        }

        Ok(IssueUrl {
            owner: owner.to_string(),
            repo: repo.to_string(),
            issue_number,
        })
    }
}

/// Fetch issue details from GitHub
pub async fn fetch_issue(owner: &str, repo: &str, issue_number: u64) -> Result<Issue> {
    let octocrab = build_octocrab()?;
    let gh_issue = octocrab.issues(owner, repo).get(issue_number).await
        .context("Failed to fetch issue from GitHub")?;
    let labels = gh_issue.labels.into_iter().map(|l| l.name).collect();
    let milestone = gh_issue.milestone.map(|m| m.title);
    let state = match gh_issue.state {
        octocrab::models::IssueState::Open => "open".to_string(),
        octocrab::models::IssueState::Closed => "closed".to_string(),
        _ => "unknown".to_string(),
    };
    Ok(Issue {
        number: gh_issue.number,
        title: gh_issue.title,
        body: gh_issue.body,
        author: gh_issue.user.login,
        created_at: gh_issue.created_at.to_rfc3339(),
        labels,
        milestone,
        state,
    })
}

/// Fetch all comments for an issue
pub async fn fetch_comments(owner: &str, repo: &str, issue_number: u64) -> Result<Vec<Comment>> {
    let octocrab = build_octocrab()?;
    let gh_comments = octocrab.issues(owner, repo).list_comments(issue_number).send().await
        .context("Failed to fetch comments from GitHub")?;

    let comments = gh_comments.into_iter().map(|c| Comment {
        body: c.body.unwrap_or_default(),
        author: c.user.login,
        created_at: c.created_at.to_rfc3339(),
        is_maintainer: false, // TODO: Enhance to check if author is a maintainer
    }).collect();
    Ok(comments)
}

/// Clone a repository (with shallow clone support)
pub fn clone_repo(owner: &str, repo: &str, shallow: bool) -> Result<PathBuf> {
    // 1. Create temp directory: std::env::temp_dir().join(format!("issuance-{}-{}", owner, repo))
    // 2. Remove if exists: std::fs::remove_dir_all
    // 3. Build git command: git clone [--depth 1 if shallow] <url> <path>
    // 4. Execute with std::process::Command and check output
    let temp_dir = std::env::temp_dir().join(format!("issuance-{}-{}", owner, repo));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).context("Failed to remove existing temp directory")?;
    }
    let repo_url = format!("https://github.com/{}/{}", owner, repo);
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone");
    if shallow {
        cmd.arg("--depth").arg("1");
    }
    cmd.arg(&repo_url).arg(&temp_dir);
    let output = cmd.output().context("Failed to execute git command")?;
    if !output.status.success() {
        return Err(anyhow!("Git clone failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(temp_dir)
}

/// Build an octocrab instance with optional authentication
fn build_octocrab() -> Result<Octocrab> {
    // 1. Load config with Config::load()?
    // 2. If token exists, use Octocrab::builder().personal_token(token)
    // 3. Otherwise use Octocrab::builder() (unauthenticated)
    // 4. Call .build()
    let config = crate::config::Config::load()?;
    let octocrab = if let Some(token) = config.github.token {
        Octocrab::builder().personal_token(token).build()?
    } else {
        Octocrab::builder().build()?
    };
    Ok(octocrab)
}
