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
    /// First-time setup: write config and install the binary
    Setup(commands::setup::SetupArgs),
    /// Clone the governance repository into the AI root
    Init(commands::init::InitArgs),
    /// Sync governance configs and/or update the binary
    Update(commands::update::UpdateArgs),
    /// Launch an AI engine (opencode/gemini/claude) with full context resolution
    Launch(commands::launch::LaunchArgs),
    /// Manage active sessions
    Session(commands::session::SessionArgs),
    /// Interact with the orbit daemon
    Daemon(commands::daemon::DaemonArgs),
}

impl Cli {
    pub fn parse_dev() -> Self {
        Self::parse()
    }
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Setup(args)) => commands::setup::run(args).await,
        Some(Commands::Init(args)) => commands::init::run(args).await,
        Some(Commands::Update(args)) => commands::update::run(args).await,
        Some(Commands::Launch(args)) => commands::launch::run(args).await,
        Some(Commands::Session(args)) => commands::session::run(args).await,
        Some(Commands::Daemon(args)) => commands::daemon::run(args).await,
        None => {
            println!("TUI not yet implemented. Use `orbit launch` to start a session.");
            Ok(())
        }
    }
}
