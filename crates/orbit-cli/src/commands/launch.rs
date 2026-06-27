use anyhow::Result;
use clap::{Args, ValueEnum};
use orbit_core::engine::Engine;
use orbit_engine::{
    config,
    launcher::{self, LaunchOptions},
    resolver,
};

/// Clap-facing engine selector. Kept separate from `orbit_core::Engine` so that
/// `orbit-core` never has to depend on clap.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliEngine {
    Opencode,
    Gemini,
    Claude,
}

impl From<CliEngine> for Engine {
    fn from(e: CliEngine) -> Self {
        match e {
            CliEngine::Opencode => Engine::Opencode,
            CliEngine::Gemini => Engine::Gemini,
            CliEngine::Claude => Engine::Claude,
        }
    }
}

#[derive(Debug, Args)]
pub struct LaunchArgs {
    /// Workspace name — case-insensitive, resolves to ~/WORKSPACE
    pub workspace: Option<String>,

    /// Tenant name within the workspace (default: AI)
    pub tenant: Option<String>,

    /// Project name within the tenant
    pub project: Option<String>,

    /// Repository name within the project
    pub repository: Option<String>,

    /// AI engine to launch [default: opencode]
    #[arg(short, long, value_enum, default_value = "opencode")]
    pub engine: CliEngine,

    /// Print the resolved config without launching the engine (useful for debugging)
    #[arg(long)]
    pub dry_run: bool,

    /// Launch the engine directly without wrapping in a tmux session
    #[arg(long)]
    pub no_tmux: bool,
}

pub async fn run(args: LaunchArgs) -> Result<()> {
    let engine = Engine::from(args.engine);

    // 1. Resolve workspace / tenant / project / repository to real paths
    let scope = resolver::resolve(resolver::ResolveArgs {
        workspace: args.workspace,
        tenant: args.tenant,
        project: args.project,
        repository: args.repository,
    })?;

    tracing::debug!(
        engine = engine.as_str(),
        work_dir = %scope.work_dir.display(),
        tenant = %scope.tenant,
        project = %scope.project,
        repository = %scope.repository,
        global_mode = scope.global_mode,
        "scope resolved"
    );

    // 2. Load and merge config from all scope layers
    let merged = config::load(&scope, engine)?;

    tracing::debug!(
        instructions = merged.instructions.len(),
        mcp_servers = merged.mcp.len(),
        "config loaded"
    );

    // 3. Dry-run: print rendered config and exit without launching
    if args.dry_run {
        let rendered = launcher::render::render(&merged, engine);
        println!("{}", serde_json::to_string_pretty(&rendered)?);
        return Ok(());
    }

    // 4. Write config file, set env vars, exec into the engine (never returns on success)
    launcher::launch(
        &scope,
        &merged,
        engine,
        LaunchOptions {
            no_tmux: args.no_tmux,
        },
    )
}
