use anyhow::Result;
use clap::{Parser, Subcommand};

mod analysis;
mod commands;
mod curation;
mod extractors;
mod journal;
mod output;
mod project_id;

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
    /// Extract prompts and output PR-ready markdown (smart defaults)
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

        /// Write output to a file instead of stdout
        #[arg(long, short = 'w', value_name = "FILE")]
        write: Option<Option<String>>,
    },

    /// Journal a prompt entry (called automatically by agent skill)
    Record {
        /// The prompt text
        #[arg(long)]
        prompt: String,

        /// Comma-separated list of files touched
        #[arg(long, value_delimiter = ',')]
        files: Vec<String>,

        /// Comma-separated list of tool calls made (e.g. Edit,Bash,Read)
        #[arg(long, value_delimiter = ',')]
        tool_calls: Vec<String>,

        /// Brief description of what was accomplished
        #[arg(long)]
        outcome: String,

        /// AI tool used (default: claude-code)
        #[arg(long, default_value = "claude-code")]
        tool: String,

        /// Model identifier (e.g. claude-sonnet-4-5)
        #[arg(long)]
        model: Option<String>,
    },

    /// Check if your AI tool is natively supported (exit 0 = yes, exit 1 = use pmtx record)
    Check,

    /// Show current project journal status
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
    Remove {
        project_id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { uncommitted, commits, since_commit, branch_lifetime, write } => {
            commands::extract::execute(uncommitted, commits, since_commit, branch_lifetime, write)?;
        }
        Commands::Record { prompt, files, tool_calls, outcome, tool, model } => {
            commands::record::execute(&prompt, files, tool_calls, &outcome, &tool, model)?;
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
