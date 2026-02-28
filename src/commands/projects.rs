use anyhow::Result;
use crate::ProjectsAction;

pub fn execute(action: ProjectsAction) -> Result<()> {
    // TODO (Phase 9): enumerate ~/.promptex/projects/, display/remove entries
    match action {
        ProjectsAction::List => println!("pmtx projects list: not yet implemented"),
        ProjectsAction::Remove { .. } => println!("pmtx projects remove: not yet implemented"),
    }
    Ok(())
}
