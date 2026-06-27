use anyhow::{Context, Result, bail};
use clap::Args;
use orbit_core::{user_config::UserConfig, workspace_config::WorkspaceConfig};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

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

        if !ai_root.join(".git").is_dir() {
            println!("  AI root is not a git repo — skipping governance sync.");
            println!("  (Run `orbit init <url>` to set up a governance repo)");
        } else if args.dry_run {
            println!(
                "  [dry-run] would run: git -C {} pull --ff-only",
                ai_root.display()
            );
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
                println!("  (Admin: set `update.binary_url` in <ai_root>/orbit.toml)");
            }
            Some(url) => {
                if args.dry_run {
                    println!();
                    println!("  [dry-run] would download binary from: {url}");
                } else {
                    update_binary(&url).await?;
                }
            }
        }
    }

    println!();
    Ok(())
}

// ── binary download ───────────────────────────────────────────────────────────

async fn update_binary(url: &str) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let tmp_path = PathBuf::from(format!("{}.tmp", current_exe.display()));

    println!("  Downloading binary from {url}...");

    let client = reqwest::Client::builder()
        .build()
        .context("failed to build HTTP client")?;

    let resp = client
        .get(url)
        .send()
        .await
        .context("HTTP request failed")?;

    if !resp.status().is_success() {
        bail!("server returned {}", resp.status());
    }

    // Capture expected checksum from response header (optional — server may omit)
    let expected_sha256 = resp
        .headers()
        .get("x-content-sha256")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Stream body to file, computing SHA-256 on the fly
    let bytes = download_with_progress(resp).await?;

    // Verify checksum if the server provided one
    if let Some(expected) = &expected_sha256 {
        let actual = sha256_hex(&bytes);
        if actual != *expected {
            bail!(
                "checksum mismatch — download may be corrupt\n  expected: {expected}\n  got:      {actual}"
            );
        }
        println!("  Checksum verified: {actual}");
    }

    // Write to temp file
    fs::write(&tmp_path, &bytes).context("failed to write temp file")?;
    fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755))?;

    // Atomic replace
    fs::rename(&tmp_path, &current_exe).context("failed to replace binary")?;

    println!("  Binary updated → {}", current_exe.display());
    println!("  Run `orbit --version` to verify.");
    Ok(())
}

async fn download_with_progress(resp: reqwest::Response) -> Result<Vec<u8>> {
    use futures_util::StreamExt;

    let total = resp.content_length();
    let mut stream = resp.bytes_stream();
    let mut buf = Vec::new();
    let mut downloaded = 0u64;
    let mut last_pct = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("error reading response body")?;
        buf.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;

        if let Some(total) = total {
            let pct = downloaded * 100 / total;
            if pct / 10 != last_pct / 10 {
                print!("\r  Downloading... {pct}%");
                use std::io::Write;
                let _ = std::io::stdout().flush();
                last_pct = pct;
            }
        }
    }

    if total.is_some() {
        println!("\r  Downloading... 100%");
    }

    Ok(buf)
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
