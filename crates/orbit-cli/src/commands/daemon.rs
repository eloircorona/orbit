use anyhow::Result;
use clap::{Args, Subcommand};
use orbit_client::ipc;
use std::time::Duration;

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Start the orbit daemon in the background
    Start,
    /// Stop the running orbit daemon
    Stop,
    /// Show daemon status
    Status,
    /// [INTERNAL] Run the daemon server in the foreground (used by `start`)
    #[command(hide = true)]
    Serve,
}

pub async fn run(args: DaemonArgs) -> Result<()> {
    match args.command {
        DaemonCommand::Start => start().await,
        DaemonCommand::Stop => stop().await,
        DaemonCommand::Status => status().await,
        DaemonCommand::Serve => serve().await,
    }
}

// ── start ─────────────────────────────────────────────────────────────────────

async fn start() -> Result<()> {
    if ipc::is_available() {
        // Try a real connection to verify it's alive, not just a stale socket
        if let Ok(info) = ipc::status().await {
            println!(
                "Daemon is already running (pid {}, uptime {}s).",
                info.pid, info.uptime_secs
            );
            return Ok(());
        }
        // Stale socket — clean it up and continue
    }

    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .args(["daemon", "serve"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()?;

    // Give the daemon a moment to bind the socket
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if ipc::is_available() {
            break;
        }
    }

    if ipc::is_available() {
        println!("Daemon started.");
    } else {
        println!("Daemon launched but socket not yet ready — it may still be starting.");
    }

    Ok(())
}

// ── stop ──────────────────────────────────────────────────────────────────────

async fn stop() -> Result<()> {
    if !ipc::is_available() {
        println!("Daemon is not running.");
        return Ok(());
    }

    ipc::shutdown().await?;
    println!("Daemon stopped.");
    Ok(())
}

// ── status ────────────────────────────────────────────────────────────────────

async fn status() -> Result<()> {
    if !ipc::is_available() {
        println!("Daemon: not running");
        return Ok(());
    }

    match ipc::status().await {
        Ok(info) => {
            let uptime = format_uptime(info.uptime_secs);
            println!("Daemon: running");
            println!("  PID:      {}", info.pid);
            println!("  Uptime:   {uptime}");
            println!("  Sessions: {} active", info.session_count);
        }
        Err(e) => {
            println!("Daemon: socket exists but not responding ({e})");
        }
    }

    Ok(())
}

// ── serve (hidden, runs the actual daemon) ────────────────────────────────────

async fn serve() -> Result<()> {
    tracing::info!("orbitd starting");
    orbit_daemon::server::run().await
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn format_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
