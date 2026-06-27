use anyhow::Result;
use clap::Args;
use orbit_core::user_config::UserConfig;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug, Args)]
pub struct SetupArgs {
    /// AI workspace root directory [default: ~/AI]
    #[arg(long)]
    pub ai_root: Option<PathBuf>,

    /// Default AI engine to use [default: opencode]
    #[arg(long)]
    pub default_engine: Option<String>,

    /// Default tenant (leave empty to always specify it explicitly)
    #[arg(long)]
    pub default_tenant: Option<String>,

    /// Directory where orbit binary is installed [default: ~/.local/bin]
    #[arg(long)]
    pub install_dir: Option<PathBuf>,

    /// Accept all defaults without prompting
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Print what would be done without writing anything
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: SetupArgs) -> Result<()> {
    println!();
    println!("  Welcome to Orbit — AI ecosystem CLI");
    println!();

    let current = UserConfig::load();

    // ── collect values (flags → interactive → default) ────────────────────────
    let ai_root = match args.ai_root {
        Some(p) => p,
        None => {
            let default = current.workspace.ai_root.to_string_lossy().into_owned();
            if args.yes {
                current.workspace.ai_root.clone()
            } else {
                ask("AI workspace root", &default)?.into()
            }
        }
    };

    let default_engine = match args.default_engine {
        Some(e) => e,
        None => {
            let default = &current.engine.default;
            if args.yes {
                default.clone()
            } else {
                ask("Default engine (opencode / gemini / claude)", default)?
            }
        }
    };

    let default_tenant = match args.default_tenant {
        Some(t) => t,
        None => {
            let default = if current.engine.default_tenant.is_empty() {
                "(none)"
            } else {
                &current.engine.default_tenant
            };
            if args.yes {
                if default == "(none)" {
                    String::new()
                } else {
                    default.to_string()
                }
            } else {
                let val = ask("Default tenant (leave blank to skip)", default)?;
                if val == "(none)" { String::new() } else { val }
            }
        }
    };

    let install_dir = match args.install_dir {
        Some(d) => d,
        None => {
            let default = current.install.dir.to_string_lossy().into_owned();
            if args.yes {
                current.install.dir.clone()
            } else {
                ask("Install directory", &default)?.into()
            }
        }
    };

    // ── build final config ────────────────────────────────────────────────────
    let mut cfg = UserConfig::default();
    cfg.workspace.ai_root = ai_root.clone();
    cfg.engine.default = default_engine.clone();
    cfg.engine.default_tenant = default_tenant.clone();
    cfg.install.dir = install_dir.clone();

    if args.dry_run {
        println!();
        println!("  [dry-run] would write {}:", UserConfig::path().display());
        println!("{}", toml::to_string_pretty(&cfg)?);
        return Ok(());
    }

    // ── save config ───────────────────────────────────────────────────────────
    cfg.save()?;
    println!();
    println!("  Config saved → {}", UserConfig::path().display());

    // ── self-install binary ───────────────────────────────────────────────────
    let install_dir_expanded = orbit_core::user_config::expand_tilde(&install_dir);
    let current_exe = std::env::current_exe()?;
    let target = install_dir_expanded.join("orbit");

    if current_exe == target {
        println!("  Binary already at {} — skipping copy", target.display());
    } else {
        fs::create_dir_all(&install_dir_expanded)?;
        fs::copy(&current_exe, &target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755))?;
        }
        println!("  Binary installed → {}", target.display());
    }

    // ── PATH hint ─────────────────────────────────────────────────────────────
    let install_dir_str = install_dir_expanded.to_string_lossy();
    let path_env = std::env::var("PATH").unwrap_or_default();
    if !path_env.split(':').any(|p| p == install_dir_str.as_ref()) {
        println!();
        println!("  Add to your shell profile:");
        println!("    export PATH=\"{install_dir_str}:$PATH\"");
    }

    // ── next steps ────────────────────────────────────────────────────────────
    println!();
    if !ai_root.exists() {
        println!("  AI root does not exist yet. To clone a governance repo:");
        println!("    orbit init <governance-url>");
    } else {
        println!("  Ready. Run `orbit launch` to start a session.");
    }
    println!();

    Ok(())
}

// ── prompt helper ─────────────────────────────────────────────────────────────

fn ask(question: &str, default: &str) -> Result<String> {
    print!("  {question} [{default}]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    Ok(if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    })
}
