use anyhow::{bail, Result};
use clap::Args;
use orbit_core::{user_config::UserConfig, workspace_config::WorkspaceConfig};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
};

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Only update governance configs (skip binary update)
    #[arg(long)]
    pub governance_only: bool,

    /// Only update the binary (skip governance sync)
    #[arg(long)]
    pub binary_only: bool,

    /// Print what would be done without making changes
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: UpdateArgs) -> Result<()> {
    let user_cfg = UserConfig::load();
    let ai_root = user_cfg.ai_root_expanded();
    let ws_cfg = WorkspaceConfig::load(&ai_root);

    let do_governance = !args.binary_only;
    let do_binary = !args.governance_only;

    // ── governance sync ───────────────────────────────────────────────────────
    if do_governance {
        if !ai_root.is_dir() {
            bail!(
                "AI root not found: {}\nRun `orbit init <url>` first.",
                ai_root.display()
            );
        }

        let is_git = ai_root.join(".git").is_dir();

        if !is_git {
            println!("  AI root is not a git repo — skipping governance sync.");
            println!("  (Run `orbit init <url>` to set up a governance repo)");
        } else if args.dry_run {
            println!("  [dry-run] would run: git -C {} pull", ai_root.display());
        } else {
            println!("  Syncing governance configs...");
            let status = Command::new("git")
                .args(["-C", &ai_root.to_string_lossy(), "pull", "--ff-only"])
                .status()?;
            if status.success() {
                println!("  Governance configs up to date.");
            } else {
                bail!("git pull failed — check your network/SSH access");
            }
        }
    }

    // ── binary update ─────────────────────────────────────────────────────────
    if do_binary {
        match ws_cfg.binary_url_for_platform() {
            None => {
                println!();
                println!("  No binary update URL configured.");
                println!("  (Company admin: set `update.binary_url` in <ai_root>/orbit.toml)");
            }
            Some(url) => {
                if args.dry_run {
                    println!();
                    println!("  [dry-run] would download binary from: {url}");
                } else {
                    update_binary(&url)?;
                }
            }
        }
    }

    println!();
    Ok(())
}

fn update_binary(url: &str) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let tmp_path = PathBuf::from(format!("{}.tmp", current_exe.display()));

    println!("  Downloading binary from {url}...");

    // Use curl; fall back to wget if unavailable
    let download_ok = try_curl(url, &tmp_path)
        .or_else(|_| try_wget(url, &tmp_path))
        .is_ok();

    if !download_ok {
        bail!("failed to download binary — curl and wget both unavailable or failed");
    }

    fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755))?;

    // Atomic replace: move tmp over the running binary
    fs::rename(&tmp_path, &current_exe)?;

    println!("  Binary updated → {}", current_exe.display());
    println!("  Run `orbit --version` to verify.");
    Ok(())
}

fn try_curl(url: &str, dest: &PathBuf) -> Result<()> {
    let status = Command::new("curl")
        .args(["-sSfL", "-o", &dest.to_string_lossy(), url])
        .status()?;
    if !status.success() {
        bail!("curl failed");
    }
    Ok(())
}

fn try_wget(url: &str, dest: &PathBuf) -> Result<()> {
    let status = Command::new("wget")
        .args(["-q", "-O", &dest.to_string_lossy(), url])
        .status()?;
    if !status.success() {
        bail!("wget failed");
    }
    Ok(())
}
