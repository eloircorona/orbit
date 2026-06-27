use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod commands;

#[derive(Debug, Parser)]
#[command(name = "orbit", about = "AI ecosystem CLI", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Launch an AI engine (opencode/gemini/claude) with full context resolution
    Launch(commands::launch::LaunchArgs),
    /// Manage active sessions
    Session(commands::session::SessionArgs),
    /// Interact with the orbit daemon
    Daemon(commands::daemon::DaemonArgs),
}

impl Cli {
    /// Entry point for `orbit-dev`: same commands + dev-only extras
    pub fn parse_dev() -> Self {
        Self::parse()
    }
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Launch(args)) => commands::launch::run(args).await,
        Some(Commands::Session(args)) => commands::session::run(args).await,
        Some(Commands::Daemon(args)) => commands::daemon::run(args).await,
        None => {
            // No subcommand → TUI (not yet implemented)
            println!("TUI not yet implemented. Use `orbit launch` to start a session.");
            Ok(())
        }
    }
}
