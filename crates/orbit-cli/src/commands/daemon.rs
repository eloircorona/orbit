use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Start the orbit daemon
    Start,
    /// Stop the orbit daemon
    Stop,
    /// Show daemon status
    Status,
}

pub async fn run(args: DaemonArgs) -> Result<()> {
    // TODO: implement daemon lifecycle management
    match args.command {
        DaemonCommand::Start => println!("Starting daemon..."),
        DaemonCommand::Stop => println!("Stopping daemon..."),
        DaemonCommand::Status => println!("Daemon status: not implemented yet."),
    }
    Ok(())
}
