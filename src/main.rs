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

    /// Format extracted JSON prompts into PR-ready markdown
    Format {
        /// Optional path to the JSON file to format. If omitted, reads from stdin.
        #[arg(value_name = "FILE")]
        file: Option<std::path::PathBuf>,

        /// Optional directory to save the formatted markdown file.
        /// If provided, `pmtx format` writes to `PROMPTS-YYYYMMDD-HHMM.md` in this directory
        /// and prints the precise file path to stdout.
        #[arg(long, value_name = "DIR")]
        out: Option<std::path::PathBuf>,

        /// If provided, `pmtx format` simply prints the current timestamp in `YYYYMMDD-HHMM` format
        /// and exits immediately without waiting for standard input.
        #[arg(long)]
        date: bool,
    },

    /// Check whether prompt extraction appears available in this repo
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
    /// Remove a project by ID or 1-based list index
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
        Commands::Format { file, out, date } => {
            commands::format::execute(file, out, date)?;
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
