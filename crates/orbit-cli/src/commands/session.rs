use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use orbit_core::session::Session;
use std::{
    io::{self, Write},
    process::Command,
};

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List all tracked sessions with their current status
    List,
    /// Send SIGTERM (or SIGKILL with --force) to a session.
    /// If no ID is given, shows an interactive selector.
    Kill {
        /// Session ID (from `orbit session list`). Omit for interactive selection.
        id: Option<String>,
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
        SessionCommand::Kill { id, force } => kill(id.as_deref(), force),
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

fn kill(id: Option<&str>, force: bool) -> Result<()> {
    let sessions = Session::load_all();
    let alive: Vec<&Session> = sessions.iter().filter(|s| s.is_running()).collect();

    let session = match id {
        Some(id) => {
            // Direct lookup by full ID or prefix
            sessions
                .iter()
                .find(|s| s.id == id || s.id.starts_with(id))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "session not found: {id}\nRun `orbit session list` to see available sessions."
                    )
                })?
        }
        None => {
            // Interactive selection from alive sessions
            if alive.is_empty() {
                println!("No active sessions to kill.");
                return Ok(());
            }
            select_session(&alive)?
        }
    };

    if !session.is_running() {
        println!(
            "Session {} is already dead. Run `orbit session clean` to remove it.",
            session.id
        );
        return Ok(());
    }

    send_signal(session, force)
}

fn select_session<'a>(alive: &[&'a Session]) -> Result<&'a Session> {
    println!("Select a session to kill:\n");
    for (i, s) in alive.iter().enumerate() {
        println!(
            "  {:>2})  {:<24}  {:<10}  {:<30}  {}",
            i + 1,
            s.id,
            s.engine,
            s.scope_label(),
            s.started_ago(),
        );
    }
    println!();

    loop {
        print!("  Enter number (1-{}): ", alive.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed.is_empty() {
            bail!("cancelled");
        }

        match trimmed.parse::<usize>() {
            Ok(n) if n >= 1 && n <= alive.len() => return Ok(alive[n - 1]),
            _ => println!("  Invalid choice — enter a number between 1 and {}.", alive.len()),
        }
    }
}

fn send_signal(session: &Session, force: bool) -> Result<()> {
    let signal = if force { "-9" } else { "-15" };
    let label = if force { "SIGKILL" } else { "SIGTERM" };

    let status = Command::new("kill")
        .args([signal, &session.pid.to_string()])
        .status()?;

    if status.success() {
        println!("Sent {label} to session {} (pid {})", session.id, session.pid);
        if !force {
            println!("Use --force to send SIGKILL if the process doesn't stop.");
        }
    } else {
        bail!(
            "kill failed — you may not have permission to signal pid {}",
            session.pid
        );
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
