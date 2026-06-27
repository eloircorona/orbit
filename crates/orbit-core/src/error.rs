use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrbitError {
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),

    #[error("tenant not found: {0}")]
    TenantNotFound(String),

    #[error("engine not supported: {0}")]
    UnsupportedEngine(String),

    #[error("daemon not running")]
    DaemonNotRunning,

    #[error("config error: {0}")]
    Config(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
