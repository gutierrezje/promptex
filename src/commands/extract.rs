use anyhow::Result;
use crate::analysis::scope::{determine_scope, ExtractionScope, ScopeFlags};

pub fn execute(
    uncommitted: bool,
    commits: Option<usize>,
    since_commit: Option<String>,
    branch_lifetime: bool,
    _write: Option<Option<String>>,
) -> Result<()> {
    let flags = ScopeFlags { uncommitted, commits, since_commit, branch_lifetime };
    let scope = determine_scope(&flags)?;

    // TODO (Phase 5+): load journal → correlate to scope → curate → output
    match &scope {
        ExtractionScope::BranchLifetime { branch, since_commit } => {
            println!("Scope: branch lifetime of '{branch}' (since {short})",
                short = &since_commit[..7]);
        }
        ExtractionScope::LastNCommits(n) => {
            println!("Scope: last {n} commit(s)");
        }
        ExtractionScope::SinceCommit(hash) => {
            println!("Scope: since commit {hash}");
        }
        ExtractionScope::Uncommitted => {
            println!("Scope: uncommitted changes only");
        }
    }

    println!("(extraction not yet implemented — phases 5-7 pending)");
    Ok(())
}
