use anyhow::{bail, Result};
use clap::Args;
use orbit_core::user_config::UserConfig;
use std::process::Command;

#[derive(Debug, Args)]
pub struct InitArgs {
    /// URL of the governance git repository
    pub governance_url: String,

    /// Branch to clone [default: main]
    #[arg(long, default_value = "main")]
    pub branch: String,

    /// Override AI root destination (default: from config)
    #[arg(long)]
    pub ai_root: Option<std::path::PathBuf>,
}

pub async fn run(args: InitArgs) -> Result<()> {
    let cfg = UserConfig::load();
    let ai_root = args
        .ai_root
        .unwrap_or_else(|| cfg.ai_root_expanded());

    if ai_root.is_dir() && ai_root.join(".git").is_dir() {
        bail!(
            "AI root already initialised: {}\nRun `orbit update` to pull the latest changes.",
            ai_root.display()
        );
    }

    if ai_root.exists() && !is_empty_dir(&ai_root) {
        bail!(
            "{} already exists and is not empty.\n\
             Remove it first or use --ai-root to choose a different path.",
            ai_root.display()
        );
    }

    println!("Cloning {} → {}", args.governance_url, ai_root.display());

    let status = Command::new("git")
        .args([
            "clone",
            "--branch", &args.branch,
            "--single-branch",
            &args.governance_url,
            &ai_root.to_string_lossy(),
        ])
        .status()?;

    if !status.success() {
        bail!("git clone failed — check the URL and your network/SSH access");
    }

    println!();
    println!("  Governance repo cloned to {}", ai_root.display());
    println!("  Run `orbit launch` to start a session.");
    println!();

    Ok(())
}

fn is_empty_dir(path: &std::path::Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut d| d.next().is_none())
        .unwrap_or(false)
}
