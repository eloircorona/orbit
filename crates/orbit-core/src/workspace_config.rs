use serde::{Deserialize, Serialize};
use std::path::Path;

// ── WorkspaceConfig ───────────────────────────────────────────────────────────

/// Workspace-level config stored in `<ai_root>/orbit.toml`.
/// Owned and distributed by the company through the governance repo.
/// Users never edit this directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub governance: GovernanceSection,
    pub update: UpdateSection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GovernanceSection {
    /// Git URL of the governance repository.
    pub url: String,
    /// Pull governance configs automatically on launch.
    pub auto_sync: bool,
    /// Minimum hours between auto-syncs (0 = every launch).
    pub sync_interval_hours: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSection {
    /// Internal URL to download new orbit binaries.
    /// Format: `http://server/orbit/latest/{platform}` where platform is
    /// `linux-x86_64`, `linux-aarch64`, `darwin-x86_64`, `darwin-aarch64`.
    pub binary_url: String,
}

// ── load ──────────────────────────────────────────────────────────────────────

impl WorkspaceConfig {
    pub fn load(ai_root: &Path) -> Self {
        let path = ai_root.join("orbit.toml");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// `true` when a governance URL has been configured.
    pub fn has_governance(&self) -> bool {
        !self.governance.url.is_empty()
    }

    /// `true` when a binary update URL has been configured.
    pub fn has_binary_url(&self) -> bool {
        !self.update.binary_url.is_empty()
    }

    /// Resolve the binary download URL for the current platform.
    pub fn binary_url_for_platform(&self) -> Option<String> {
        if self.update.binary_url.is_empty() {
            return None;
        }
        let platform = current_platform();
        let base = self.update.binary_url.trim_end_matches('/');
        Some(format!("{base}/{platform}"))
    }
}

fn current_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        _ => "linux-x86_64", // safe fallback
    }
}
