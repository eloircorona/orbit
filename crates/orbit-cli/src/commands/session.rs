use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use orbit_core::session::Session;
use std::process::Command;

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List all tracked sessions with their current status
    List,
    /// Send SIGTERM (or SIGKILL with --force) to a session
    Kill {
        /// Session ID (from `orbit session list`)
        id: String,
        /// Use SIGKILL instead of SIGTERM
        #[arg(long, short)]
        force: bool,
    },
    /// Remove session files for processes that are no longer running
    Clean,
}

pub async fn run(args: SessionArgs) -> Result<()> {
    match args.command {
        SessionCommand::List => list(),
        SessionCommand::Kill { id, force } => kill(&id, force),
        SessionCommand::Clean => clean(),
    }
}

// ── list ──────────────────────────────────────────────────────────────────────

fn list() -> Result<()> {
    let sessions = Session::load_all();

    if sessions.is_empty() {
        println!("No tracked sessions.");
        return Ok(());
    }

    // Column widths
    let id_w = sessions.iter().map(|s| s.id.len()).max().unwrap_or(10).max(10);
    let eng_w = 10usize;
    let scope_w = sessions.iter().map(|s| s.scope_label().len()).max().unwrap_or(20).max(20);

    println!(
        "{:<id_w$}  {:<eng_w$}  {:<scope_w$}  {:<6}  {}",
        "ID", "ENGINE", "SCOPE", "STATUS", "STARTED",
        id_w = id_w, eng_w = eng_w, scope_w = scope_w,
    );
    println!("{}", "-".repeat(id_w + eng_w + scope_w + 30));

    for s in &sessions {
        let status = if s.is_running() { "alive " } else { "dead  " };
        println!(
            "{:<id_w$}  {:<eng_w$}  {:<scope_w$}  {}  {}",
            s.id, s.engine, s.scope_label(), status, s.started_ago(),
            id_w = id_w, eng_w = eng_w, scope_w = scope_w,
        );
    }

    Ok(())
}

// ── kill ──────────────────────────────────────────────────────────────────────

fn kill(id: &str, force: bool) -> Result<()> {
    let sessions = Session::load_all();
    let session = sessions.iter().find(|s| s.id == id || s.id.starts_with(id));

    let Some(s) = session else {
        bail!("session not found: {id}\nRun `orbit session list` to see available sessions.");
    };

    if !s.is_running() {
        println!("Session {id} is already dead. Run `orbit session clean` to remove it.");
        return Ok(());
    }

    let signal = if force { "-9" } else { "-15" };
    let label = if force { "SIGKILL" } else { "SIGTERM" };

    let status = Command::new("kill")
        .args([signal, &s.pid.to_string()])
        .status()?;

    if status.success() {
        println!("Sent {label} to session {} (pid {})", s.id, s.pid);
        if !force {
            println!("Use --force to send SIGKILL if the process doesn't stop.");
        }
    } else {
        bail!("kill failed — you may not have permission to signal pid {}", s.pid);
    }

    Ok(())
}

// ── clean ─────────────────────────────────────────────────────────────────────

fn clean() -> Result<()> {
    let sessions = Session::load_all();
    let dead: Vec<_> = sessions.iter().filter(|s| !s.is_running()).collect();

    if dead.is_empty() {
        println!("Nothing to clean — all tracked sessions are alive.");
        return Ok(());
    }

    for s in &dead {
        s.delete()?;
        println!("Removed dead session {} (pid {} / {})", s.id, s.pid, s.engine);
    }
    println!("Cleaned {} session file(s).", dead.len());

    Ok(())
}
