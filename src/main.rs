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

        /// Enhance the context pack using local AI tools
        #[arg(long)]
        enhance: bool,
    },

    /// Analyze repository contribution culture
    /// Remove .issuance/ folder
    Clean,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Grab { url, enhance } => {
            commands::grab::execute(&url, enhance).await?;
        }
        Commands::Clean => {
            commands::clean::execute()?;
        }
    }

    Ok(())
}
