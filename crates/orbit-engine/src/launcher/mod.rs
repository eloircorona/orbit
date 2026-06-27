pub mod agents;
pub mod render;
pub mod runtime;

use anyhow::{bail, Result};
use orbit_core::{context::OrbitScope, engine::Engine, session::Session};
use std::{fs, os::unix::process::CommandExt, path::Path, process::Command};

use crate::config::MergedConfig;

/// Full launch sequence:
/// 1. Create runtime directory structure
/// 2. Materialise agent/command files for the engine
/// 3. Render and write the engine config file
/// 4. Export environment variables
/// 5. `exec` into the engine — replaces the current process (no return on success)
pub fn launch(scope: &OrbitScope, config: &MergedConfig, engine: Engine) -> Result<()> {
    // 1. Runtime dirs
    let paths = runtime::setup(scope, engine)?;

    // 2. Agent materialisation (.opencode/agents/ or .claude/agents/ + CLAUDE.md)
    agents::build(scope, engine, &paths.runtime_dir, &config.instructions)?;

    // 3. Write config file
    let rendered = render::render(config, engine);
    let json_str = serde_json::to_string_pretty(&rendered)?;
    fs::write(&paths.config_file, json_str)?;

    // 4. Register session — must happen BEFORE set_env() overwrites XDG_DATA_HOME.
    //    Session::sessions_dir() reads the real system XDG_DATA_HOME here.
    let session = Session::new(
        std::process::id(),
        engine.as_str(),
        &scope.tenant,
        &scope.project,
        &scope.repository,
        scope.work_dir.clone(),
        scope.global_mode,
    );
    if let Err(e) = session.save() {
        tracing::warn!("could not save session: {e}");
    }

    // 5. Environment variables (overrides XDG dirs — do this last before exec)
    set_env(scope, engine, &paths);

    // 6. cd into work_dir and exec the engine (never returns on success)
    std::env::set_current_dir(&scope.work_dir)?;
    exec_engine(engine, &paths.config_file)
}

/// Set the environment variables the engine expects.
///
/// # Safety
/// `set_var` is unsafe in Rust 1.80+ because it is not thread-safe.
/// This is safe here: the launcher runs in a single-threaded context and
/// sets env vars immediately before `exec`-ing the engine — no other threads
/// are reading the environment concurrently.
fn set_env(scope: &OrbitScope, engine: Engine, paths: &runtime::RuntimePaths) {
    unsafe {
        // XDG isolation — keeps each engine/tenant in its own runtime
        std::env::set_var("XDG_CONFIG_HOME", &paths.xdg_config_home);
        std::env::set_var("XDG_DATA_HOME", &paths.xdg_data);
        std::env::set_var("XDG_CACHE_HOME", &paths.xdg_cache);
        std::env::set_var("XDG_STATE_HOME", &paths.xdg_state);

        // Orbit scope — available to the engine and any hooks/scripts it runs
        std::env::set_var("AI_ENGINE", engine.as_str());
        std::env::set_var("AI_WORKSPACE_ROOT", scope.workspace_root.to_string_lossy().as_ref());
        std::env::set_var("AI_CONTEXT_ROOT", scope.ai_context_root.to_string_lossy().as_ref());
        std::env::set_var("AI_GLOBAL_ROOT", scope.global_ai_root.to_string_lossy().as_ref());
        std::env::set_var("AI_TENANT", &scope.tenant);
        std::env::set_var("AI_PROJECT", &scope.project);
        std::env::set_var("AI_REPOSITORY", &scope.repository);
        std::env::set_var("AI_GLOBAL_MODE", if scope.global_mode { "1" } else { "0" });

        // Engine-specific config pointers
        match engine {
            Engine::Opencode => {
                std::env::set_var("OPENCODE_CONFIG", &paths.config_file);
            }
            Engine::Gemini => {
                std::env::set_var("GEMINI_CLI_HOME", &paths.runtime_dir);
                std::env::set_var("GEMINI_CLI_SYSTEM_SETTINGS_PATH", &paths.config_file);
            }
            Engine::Claude => {
                // Claude reads auth from ~/.claude — no CLAUDE_CONFIG_DIR override needed.
                // MCPs are passed via --mcp-config at exec time (see exec_engine below).
            }
        }
    }
}

/// Replace the current process with the engine binary.
/// On success this never returns — the OS replaces us with the engine.
fn exec_engine(engine: Engine, config_file: &Path) -> Result<()> {
    let mut cmd = match engine {
        Engine::Claude => {
            let mut c = Command::new("claude");
            c.arg("--mcp-config").arg(config_file);
            c
        }
        Engine::Opencode | Engine::Gemini => Command::new(engine.as_str()),
    };

    let err = cmd.exec(); // replaces the process; only returns on error
    bail!("failed to exec {}: {}", engine.as_str(), err);
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MergedConfig, McpServer};
    use orbit_core::context::OrbitScope;
    use std::{collections::HashMap, fs, path::PathBuf};
    use tempfile::TempDir;

    fn minimal_scope(tmp: &TempDir) -> OrbitScope {
        let root = tmp.path().to_path_buf();
        OrbitScope {
            workspace_root: root.clone(),
            ai_context_root: root.clone(),
            global_ai_root: root.clone(),
            tenant_dir: root.clone(),
            code_root: root.clone(),
            work_dir: root.clone(),
            global_mode: true,
            ..Default::default()
        }
    }

    fn config_with_mcp() -> MergedConfig {
        let mut cfg = MergedConfig::default();
        cfg.mcp.insert(
            "my-server".into(),
            McpServer {
                command: vec!["npx".into(), "-y".into(), "some-mcp".into()],
                environment: HashMap::from([("KEY".into(), "val".into())]),
                cwd: None,
                server_type: "local".into(),
            },
        );
        cfg.instructions.push(PathBuf::from("/fake/README.md"));
        cfg
    }

    #[test]
    fn setup_creates_runtime_dirs() {
        let tmp = TempDir::new().unwrap();
        let scope = minimal_scope(&tmp);
        let paths = runtime::setup(&scope, Engine::Opencode).unwrap();
        assert!(paths.xdg_data.is_dir());
        assert!(paths.xdg_cache.is_dir());
        assert!(paths.xdg_state.is_dir());
    }

    #[test]
    fn writes_opencode_config_file() {
        let tmp = TempDir::new().unwrap();
        let scope = minimal_scope(&tmp);
        let cfg = config_with_mcp();
        let paths = runtime::setup(&scope, Engine::Opencode).unwrap();
        let rendered = render::render(&cfg, Engine::Opencode);
        fs::write(&paths.config_file, serde_json::to_string_pretty(&rendered).unwrap()).unwrap();

        assert!(paths.config_file.exists());
        let content = fs::read_to_string(&paths.config_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["mcp"]["my-server"]["command"].is_array());
    }

    #[test]
    fn writes_claude_mcp_config() {
        let tmp = TempDir::new().unwrap();
        let scope = minimal_scope(&tmp);
        let cfg = config_with_mcp();
        let paths = runtime::setup(&scope, Engine::Claude).unwrap();
        let rendered = render::render(&cfg, Engine::Claude);
        fs::write(&paths.config_file, serde_json::to_string_pretty(&rendered).unwrap()).unwrap();

        let content = fs::read_to_string(&paths.config_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        // Claude config must only have mcpServers
        assert_eq!(parsed.as_object().unwrap().len(), 1);
        assert!(parsed["mcpServers"].is_object());
    }

    #[test]
    fn runtime_paths_differ_per_engine() {
        let tmp = TempDir::new().unwrap();
        let scope = minimal_scope(&tmp);
        let oc = runtime::setup(&scope, Engine::Opencode).unwrap();
        let cl = runtime::setup(&scope, Engine::Claude).unwrap();
        assert_ne!(oc.runtime_dir, cl.runtime_dir);
    }
}
