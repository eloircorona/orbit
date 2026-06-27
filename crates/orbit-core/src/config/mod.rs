use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrbitConfig {
    pub instructions: Vec<PathBuf>,
    pub mcp: std::collections::HashMap<String, serde_json::Value>,
}
