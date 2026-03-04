use anyhow::Result;
use clap::{Parser, Subcommand};

mod analysis;
mod commands;
mod curation;
mod extractors;
mod output;
mod project_id;
mod prompt;

#[derive(Parser)]
#[command(name = "pmtx")]
#[command(about = "Extract and curate AI prompts for your pull requests", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract prompts and output structured JSON for agent-side rendering
    Extract {
        /// Only extract prompts for uncommitted changes
        #[arg(long)]
        uncommitted: bool,

        /// Extract prompts for the last N commits
        #[arg(long, value_name = "N")]
        commits: Option<usize>,

        /// Extract prompts since a specific commit hash
        #[arg(long, value_name = "HASH")]
        since_commit: Option<String>,

        /// Extract prompts for the full branch lifetime
        #[arg(long)]
        branch_lifetime: bool,

        /// Extract prompts from the last duration (e.g. 2h, 1d, 3w)
        #[arg(long, value_name = "DURATION")]
        since: Option<String>,
    },

    /// Check if your AI tool is natively supported (exit 0 = yes, exit 1 = unsupported)
    Check,

    /// Show current project prompt extraction status
    Status,

    /// Manage tracked projects
    Projects {
        #[command(subcommand)]
        action: ProjectsAction,
    },
}

#[derive(Subcommand)]
enum ProjectsAction {
    /// List all tracked projects
    List,
    /// Remove a specific project by ID
    Remove { project_id: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            uncommitted,
            commits,
            since_commit,
            branch_lifetime,
            since,
        } => {
            commands::extract::execute(uncommitted, commits, since_commit, branch_lifetime, since)?;
        }
        Commands::Check => {
            commands::check::execute()?;
        }
        Commands::Status => {
            commands::status::execute()?;
        }
        Commands::Projects { action } => {
            commands::projects::execute(action)?;
        }
    }

    Ok(())
}
