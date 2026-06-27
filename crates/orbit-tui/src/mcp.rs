use anyhow::Result;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct McpEntry {
    pub name: String,
    pub command_display: String,
    pub scope: String,
    pub source_file: PathBuf,
}

pub fn load_entries(ai_root: &Path, default_tenant: &str) -> Vec<McpEntry> {
    let mut entries = Vec::new();
    collect_from_file(ai_root.join("mcp.json"), "global", &mut entries);
    if !default_tenant.is_empty() {
        let path = ai_root
            .join("tenants")
            .join(default_tenant)
            .join("mcp.json");
        collect_from_file(path, &format!("tenant/{default_tenant}"), &mut entries);
    }
    entries
}

fn collect_from_file(path: PathBuf, scope: &str, entries: &mut Vec<McpEntry>) {
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(servers) = json.get("mcpServers").and_then(|v| v.as_object()) else {
        return;
    };
    for (name, server) in servers {
        let cmd = server
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("?");
        let first_args: Vec<&str> = server
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).take(2).collect())
            .unwrap_or_default();
        let command_display = if first_args.is_empty() {
            cmd.to_string()
        } else {
            format!("{cmd} {}", first_args.join(" "))
        };
        entries.push(McpEntry {
            name: name.clone(),
            command_display,
            scope: scope.to_string(),
            source_file: path.clone(),
        });
    }
}

pub fn add_server(
    path: &Path,
    name: &str,
    command: &str,
    args: &[String],
    env: HashMap<String, String>,
) -> Result<()> {
    let mut json: serde_json::Value = if path.is_file() {
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !json.get("mcpServers").map_or(false, |v| v.is_object()) {
        json["mcpServers"] = serde_json::json!({});
    }

    let mut server = serde_json::json!({ "command": command });
    if !args.is_empty() {
        server["args"] = serde_json::Value::Array(
            args.iter()
                .map(|a| serde_json::Value::String(a.clone()))
                .collect(),
        );
    }
    if !env.is_empty() {
        let env_obj: serde_json::Map<_, _> = env
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect();
        server["env"] = serde_json::Value::Object(env_obj);
    }

    json["mcpServers"][name] = server;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
    Ok(())
}

pub fn remove_server(path: &Path, name: &str) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(path)?;
    let mut json: serde_json::Value = serde_json::from_str(&text)?;
    if let Some(servers) = json.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove(name);
    }
    std::fs::write(path, serde_json::to_string_pretty(&json)?)?;
    Ok(())
}
