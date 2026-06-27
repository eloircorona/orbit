use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List active sessions
    List,
    /// Stop a session by ID
    Stop { id: String },
}

pub async fn run(args: SessionArgs) -> Result<()> {
    // TODO: connect to daemon and manage sessions
    match args.command {
        SessionCommand::List => println!("No active sessions."),
        SessionCommand::Stop { id } => println!("Stopping session: {id}"),
    }
    Ok(())
}
