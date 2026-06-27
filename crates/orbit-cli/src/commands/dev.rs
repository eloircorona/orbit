use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use orbit_core::{user_config::UserConfig, workspace_config::WorkspaceConfig};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

#[derive(Debug, Args)]
pub struct DevArgs {
    #[command(subcommand)]
    pub command: DevCommand,
}

#[derive(Debug, Subcommand)]
pub enum DevCommand {
    /// Activate development mode (orbit → orbit-dev)
    Enable {
        /// Development token issued by the project admin
        #[arg(long, short)]
        token: String,
    },
    /// Deactivate development mode (restore stable orbit)
    Disable,
    /// Show current mode and installation paths
    Status,
    /// [ADMIN] Generate a new token + its hash for governance config
    GenerateToken,
}

pub async fn run(args: DevArgs) -> Result<()> {
    match args.command {
        DevCommand::Enable { token } => enable(&token),
        DevCommand::Disable => disable(),
        DevCommand::Status => status(),
        DevCommand::GenerateToken => generate_token(),
    }
}

// ── enable ────────────────────────────────────────────────────────────────────

fn enable(token: &str) -> Result<()> {
    let user_cfg = UserConfig::load();
    let ai_root = user_cfg.ai_root_expanded();
    let ws_cfg = WorkspaceConfig::load(&ai_root);

    // 1. Check governance has a token hash configured
    if !ws_cfg.dev.is_configured() {
        bail!(
            "No dev token configured in governance.\n\
             Admin: run `orbit dev generate-token` and add the hash to\n\
             <ai_root>/orbit.toml under [dev] token_hash = \"sha256:...\""
        );
    }

    // 2. Verify token
    if !verify_token(token, &ws_cfg.dev.token_hash) {
        bail!("Invalid token — access denied.");
    }

    // 3. Resolve paths
    let install_dir = user_cfg.install_dir_expanded();
    let orbit_path = install_dir.join("orbit");
    let orbit_dev_path = install_dir.join("orbit-dev");
    let orbit_stable_path = install_dir.join("orbit.stable");

    if !orbit_dev_path.exists() {
        bail!(
            "orbit-dev not found at {}\n\
             Run `make install` from the orbit source directory first.",
            orbit_dev_path.display()
        );
    }

    // 4. Already in dev mode?
    if is_dev_symlink(&orbit_path, &orbit_dev_path) {
        println!("Already in dev mode — orbit → orbit-dev");
        return Ok(());
    }

    // 5. Back up stable binary (skip if already backed up)
    if !orbit_stable_path.exists() {
        fs::copy(&orbit_path, &orbit_stable_path)?;
    }

    // 6. Replace orbit with symlink → orbit-dev
    fs::remove_file(&orbit_path)?;
    std::os::unix::fs::symlink(&orbit_dev_path, &orbit_path)?;

    println!("Dev mode enabled.");
    println!("  {} → {}", orbit_path.display(), orbit_dev_path.display());
    println!(
        "  Stable binary backed up at {}",
        orbit_stable_path.display()
    );
    println!();
    println!("  Run `orbit dev disable` to restore the stable binary.");

    Ok(())
}

// ── disable ───────────────────────────────────────────────────────────────────

fn disable() -> Result<()> {
    let install_dir = UserConfig::load().install_dir_expanded();
    let orbit_path = install_dir.join("orbit");
    let orbit_stable_path = install_dir.join("orbit.stable");

    if !is_symlink(&orbit_path) {
        println!("Already in stable mode — nothing to do.");
        return Ok(());
    }

    if !orbit_stable_path.exists() {
        bail!(
            "Stable backup not found at {}.\n\
             Re-install orbit: make install",
            orbit_stable_path.display()
        );
    }

    // Remove symlink, restore backup
    fs::remove_file(&orbit_path)?;
    fs::copy(&orbit_stable_path, &orbit_path)?;
    fs::remove_file(&orbit_stable_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&orbit_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("Dev mode disabled — stable binary restored.");

    Ok(())
}

// ── status ────────────────────────────────────────────────────────────────────

fn status() -> Result<()> {
    let install_dir = UserConfig::load().install_dir_expanded();
    let orbit_path = install_dir.join("orbit");
    let orbit_dev_path = install_dir.join("orbit-dev");

    println!("Install directory: {}", install_dir.display());
    println!();

    if is_dev_symlink(&orbit_path, &orbit_dev_path) {
        println!("  Mode:  dev (orbit → orbit-dev)");
        if let Ok(target) = orbit_path.read_link() {
            println!("  Links: {} → {}", orbit_path.display(), target.display());
        }
    } else if orbit_path.exists() {
        println!("  Mode:  stable");
        println!("  Binary: {}", orbit_path.display());
    } else {
        println!("  orbit not found at {}", orbit_path.display());
        println!("  Run `orbit setup` to install.");
    }

    if orbit_dev_path.exists() {
        println!("  orbit-dev: {} (present)", orbit_dev_path.display());
    } else {
        println!("  orbit-dev: not installed");
    }

    Ok(())
}

// ── generate-token ────────────────────────────────────────────────────────────

fn generate_token() -> Result<()> {
    let (token, hash) = create_token()?;

    println!("  Token (distribute to developers — keep secret):");
    println!("    {token}");
    println!();
    println!("  Hash  (add to <ai_root>/orbit.toml):");
    println!("    [dev]");
    println!("    token_hash = \"{hash}\"");
    println!();
    println!("  To revoke access: change token_hash in the governance repo.");

    Ok(())
}

// ── crypto helpers ────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn verify_token(token: &str, stored_hash: &str) -> bool {
    let computed = format!("sha256:{}", sha256_hex(token.as_bytes()));
    // Constant-length comparison to avoid timing leaks
    constant_time_eq(computed.as_bytes(), stored_hash.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn create_token() -> Result<(String, String)> {
    let mut bytes = [0u8; 32];
    let mut f = fs::File::open("/dev/urandom")?;
    f.read_exact(&mut bytes)?;
    let token = hex::encode(bytes);
    let hash = format!("sha256:{}", sha256_hex(token.as_bytes()));
    Ok((token, hash))
}

// ── path helpers ──────────────────────────────────────────────────────────────

fn is_symlink(path: &PathBuf) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

fn is_dev_symlink(orbit: &PathBuf, orbit_dev: &PathBuf) -> bool {
    if !is_symlink(orbit) {
        return false;
    }
    orbit
        .read_link()
        .map(|target| {
            // Compare resolved paths
            let target_abs = if target.is_absolute() {
                target
            } else {
                orbit
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join(target)
            };
            target_abs == *orbit_dev
        })
        .unwrap_or(false)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_verify_roundtrip() {
        let (token, hash) = create_token().unwrap();
        assert!(verify_token(&token, &hash));
    }

    #[test]
    fn wrong_token_rejected() {
        let (_, hash) = create_token().unwrap();
        assert!(!verify_token("wrong-token", &hash));
    }

    #[test]
    fn hash_format_is_sha256_prefixed() {
        let (_, hash) = create_token().unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), "sha256:".len() + 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"abc", b"xyz"));
        assert!(!constant_time_eq(b"short", b"longer_string"));
    }
}
