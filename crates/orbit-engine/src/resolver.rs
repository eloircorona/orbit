use anyhow::{bail, Result};
use directories::BaseDirs;
use orbit_core::{context::OrbitScope, user_config::UserConfig};
use std::path::{Path, PathBuf};

/// Arguments for scope resolution — direct mapping from CLI flags.
#[derive(Debug, Default)]
pub struct ResolveArgs {
    pub workspace: Option<String>,
    pub tenant: Option<String>,
    pub project: Option<String>,
    pub repository: Option<String>,
}

/// Public entry point. Resolves args against the real filesystem.
/// Reads `ai_root` from `~/.config/orbit/config.toml` (falls back to `~/AI`).
pub fn resolve(args: ResolveArgs) -> Result<OrbitScope> {
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let ai_root = UserConfig::load().ai_root_expanded();
    resolve_inner(args, base_dirs.home_dir(), &ai_root)
}

/// Testable core: accepts explicit home and ai_root paths.
pub fn resolve_with_home(args: ResolveArgs, home: &Path) -> Result<OrbitScope> {
    let ai_root = home.join("AI");
    resolve_inner(args, home, &ai_root)
}

/// Testable core with fully explicit paths.
pub fn resolve_with_roots(args: ResolveArgs, home: &Path, ai_root: &Path) -> Result<OrbitScope> {
    resolve_inner(args, home, ai_root)
}

fn resolve_inner(args: ResolveArgs, home: &Path, ai_root: &Path) -> Result<OrbitScope> {
    // ── global mode: no arguments at all ─────────────────────────────────────
    if args.workspace.is_none() {
        if !ai_root.is_dir() {
            bail!(
                "AI root not found: {}\nRun `orbit setup` to configure or `orbit init` to clone the governance repo.",
                ai_root.display()
            );
        }
        return Ok(OrbitScope {
            ai_context_root: ai_root.to_path_buf(),
            global_ai_root: ai_root.to_path_buf(),
            code_root: ai_root.to_path_buf(),
            work_dir: ai_root.to_path_buf(),
            workspace_root: ai_root.to_path_buf(),
            global_mode: true,
            ..Default::default()
        });
    }

    // ── resolve workspace root ────────────────────────────────────────────────
    let workspace_str = args.workspace.as_deref().unwrap();
    let workspace_root = find_dir_icase(home, workspace_str)
        .ok_or_else(|| anyhow::anyhow!("workspace not found: {workspace_str}"))?;

    // AI_CONTEXT_ROOT: prefer WORKSPACE_ROOT/<ai_root_name> when it has tenants/
    let ai_root_name = ai_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "AI".to_string());

    let ai_context_root = {
        let candidate = workspace_root.join(&ai_root_name);
        if candidate.join("tenants").is_dir() {
            candidate
        } else {
            workspace_root.clone()
        }
    };

    let tenants_root = ai_context_root.join("tenants");

    // ── resolve tenant ────────────────────────────────────────────────────────
    let user_cfg = UserConfig::load();
    let default_tenant = if user_cfg.engine.default_tenant.is_empty() {
        "AI"
    } else {
        &user_cfg.engine.default_tenant
    };
    let tenant_input = args.tenant.as_deref().unwrap_or(default_tenant);
    let tenant = resolve_name(&tenants_root, tenant_input);

    // ── resolve project ───────────────────────────────────────────────────────
    let project = match &args.project {
        None => String::new(),
        Some(p) => {
            let projects_root = tenants_root.join(&tenant).join("projects");
            resolve_name(&projects_root, p)
        }
    };

    // ── resolve repository ────────────────────────────────────────────────────
    let repository = match (&args.repository, project.is_empty()) {
        (Some(r), false) => {
            let repos_root = tenants_root
                .join(&tenant)
                .join("projects")
                .join(&project)
                .join("repositories");
            resolve_name(&repos_root, r)
        }
        _ => String::new(),
    };

    // ── derive paths ──────────────────────────────────────────────────────────
    let code_root = workspace_root.join(&tenant);
    let tenant_dir = ai_context_root.join("tenants").join(&tenant);

    let work_dir = if !repository.is_empty() {
        code_root.join(&project).join(&repository)
    } else if !project.is_empty() {
        code_root.join(&project)
    } else {
        code_root.clone()
    };

    Ok(OrbitScope {
        workspace_root,
        ai_context_root,
        global_ai_root: ai_root.to_path_buf(),
        tenant,
        project,
        repository,
        tenant_dir,
        code_root,
        work_dir,
        global_mode: false,
    })
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Find a direct subdirectory of `root` whose name matches `target` ignoring case.
/// Returns the original on-disk name, or `target` itself if not found.
fn resolve_name(root: &Path, target: &str) -> String {
    find_dir_icase(root, target)
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| target.to_string())
}

/// Find a direct subdirectory of `root` whose lowercased name equals
/// `target.to_lowercase()`. Returns the full path on success.
fn find_dir_icase(root: &Path, target: &str) -> Option<PathBuf> {
    let needle = target.to_lowercase();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase() == needle)
                .unwrap_or(false)
        })
        .collect();

    entries.sort();
    entries.into_iter().next()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fake_home() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path()
            .join("AI/tenants/AIDEV/projects/AI-ECOSYSTEM/repositories/orbit");
        fs::create_dir_all(&repo).unwrap();
        tmp
    }

    #[test]
    fn global_mode_opens_ai_root() {
        let home = fake_home();
        let scope = resolve_with_home(ResolveArgs::default(), home.path()).unwrap();
        assert!(scope.global_mode);
        assert_eq!(scope.workspace_root, home.path().join("AI"));
        assert!(scope.tenant.is_empty());
    }

    #[test]
    fn resolves_workspace_case_insensitive() {
        let home = fake_home();
        let args = ResolveArgs { workspace: Some("ai".to_string()), ..Default::default() };
        let scope = resolve_with_home(args, home.path()).unwrap();
        assert_eq!(scope.workspace_root, home.path().join("AI"));
    }

    #[test]
    fn resolves_tenant_case_insensitive() {
        let home = fake_home();
        let args = ResolveArgs {
            workspace: Some("ai".to_string()),
            tenant: Some("aidev".to_string()),
            ..Default::default()
        };
        let scope = resolve_with_home(args, home.path()).unwrap();
        assert_eq!(scope.tenant, "AIDEV");
    }

    #[test]
    fn resolves_full_scope() {
        let home = fake_home();
        let args = ResolveArgs {
            workspace: Some("AI".to_string()),
            tenant: Some("AIDEV".to_string()),
            project: Some("ai-ecosystem".to_string()),
            repository: Some("ORBIT".to_string()),
        };
        let scope = resolve_with_home(args, home.path()).unwrap();
        assert_eq!(scope.tenant, "AIDEV");
        assert_eq!(scope.project, "AI-ECOSYSTEM");
        assert_eq!(scope.repository, "orbit");
        assert_eq!(scope.work_dir, home.path().join("AI/AIDEV/AI-ECOSYSTEM/orbit"));
    }

    #[test]
    fn missing_workspace_returns_error() {
        let home = fake_home();
        let args = ResolveArgs { workspace: Some("nonexistent".to_string()), ..Default::default() };
        assert!(resolve_with_home(args, home.path()).is_err());
    }

    #[test]
    fn work_dir_falls_back_to_project_when_no_repo() {
        let home = fake_home();
        let args = ResolveArgs {
            workspace: Some("AI".to_string()),
            tenant: Some("AIDEV".to_string()),
            project: Some("AI-ECOSYSTEM".to_string()),
            repository: None,
        };
        let scope = resolve_with_home(args, home.path()).unwrap();
        assert_eq!(scope.work_dir, home.path().join("AI/AIDEV/AI-ECOSYSTEM"));
    }

    #[test]
    fn custom_ai_root_is_used() {
        let home = TempDir::new().unwrap();
        let ai_root = home.path().join("MyAI");
        fs::create_dir_all(ai_root.join("tenants/CORP/projects/P/repositories/R")).unwrap();

        let args = ResolveArgs::default();
        let scope = resolve_with_roots(args, home.path(), &ai_root).unwrap();
        assert!(scope.global_mode);
        assert_eq!(scope.global_ai_root, ai_root);
    }
}
