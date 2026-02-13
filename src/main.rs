use clap::{Parser, Subcommand};
use anyhow::Result;

mod config;
mod commands;
mod services;

#[derive(Parser)]
#[command(name = "issuance")]
#[command(about = "A context orchestrator for AI-assisted open source contributions", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch an issue and generate context pack
    Grab {
        /// GitHub issue URL (e.g., https://github.com/owner/repo/issues/123)
        url: String,

        /// Optional clone directory (defaults to repository name)
        directory: Option<String>,
    },

    /// Remove .issuance/ folder
    Clean,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Grab { url, directory } => {
            commands::grab::execute(&url, directory.as_deref()).await?;
        }
        Commands::Clean => {
            commands::clean::execute()?;
        }
    }

    Ok(())
}
