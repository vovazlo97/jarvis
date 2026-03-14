use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Permissions a plugin requests at install time.
/// All default to false (deny-by-default).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginPermissions {
    #[serde(default)]
    pub filesystem: bool,
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub processes: bool,
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self {
            filesystem: false,
            network: false,
            processes: false,
        }
    }
}

/// Validated plugin manifest loaded from `plugin.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub permissions: PluginPermissions,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
}

impl PluginManifest {
    /// Validate that required fields are non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("plugin.json: 'id' must not be empty".to_string());
        }
        if self.version.trim().is_empty() {
            return Err("plugin.json: 'version' must not be empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("plugin.json: 'name' must not be empty".to_string());
        }
        Ok(())
    }
}

/// Parse and validate a `plugin.json` file.
pub fn load(path: &Path) -> Result<PluginManifest, String> {
    let json = fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))?;
    let manifest: PluginManifest =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;
    manifest.validate()?;
    Ok(manifest)
}

/// Scan a directory for `plugin.json` files one level deep.
/// Skips entries that fail to parse or validate (logs a warning).
pub fn scan_plugins_dir(dir: &Path) -> Vec<PluginManifest> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let manifest_path = e.path().join("plugin.json");
            match load(&manifest_path) {
                Ok(m) => Some(m),
                Err(err) => {
                    warn!("Skipping plugin {:?}: {}", e.path(), err);
                    None
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn minimal_manifest_json() -> &'static str {
        r#"{
            "id": "test-plugin",
            "version": "1.0.0",
            "name": "Test Plugin",
            "description": "A test plugin",
            "author": "tester"
        }"#
    }

    #[test]
    fn parse_minimal_manifest_succeeds() {
        let m: PluginManifest = serde_json::from_str(minimal_manifest_json()).unwrap();
        assert_eq!(m.id, "test-plugin");
        assert_eq!(m.version, "1.0.0");
        assert!(!m.permissions.filesystem);
        assert!(m.commands.is_empty());
        assert!(m.agents.is_empty());
        assert!(m.endpoint.is_none());
    }

    #[test]
    fn parse_full_manifest_succeeds() {
        let json = r#"{
            "id": "full-plugin",
            "version": "0.2.0",
            "name": "Full Plugin",
            "description": "Full featured plugin",
            "author": "dev",
            "commands": ["open-browser", "search-web"],
            "agents": ["browser-agent"],
            "capabilities": ["automation", "web"],
            "permissions": { "filesystem": false, "network": true, "processes": false },
            "endpoint": "http://localhost:8080",
            "signature": "abc123"
        }"#;
        let m: PluginManifest = serde_json::from_str(json).unwrap();
        assert!(m.permissions.network);
        assert_eq!(m.commands, vec!["open-browser", "search-web"]);
        assert_eq!(m.endpoint, Some("http://localhost:8080".to_string()));
    }

    #[test]
    fn validate_rejects_empty_id() {
        let m = PluginManifest {
            id: "".to_string(),
            version: "1.0.0".to_string(),
            name: "X".to_string(),
            description: "".to_string(),
            author: "".to_string(),
            commands: vec![],
            agents: vec![],
            capabilities: vec![],
            permissions: PluginPermissions::default(),
            endpoint: None,
            signature: None,
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_version() {
        let m = PluginManifest {
            id: "ok-id".to_string(),
            version: "".to_string(),
            name: "X".to_string(),
            description: "".to_string(),
            author: "".to_string(),
            commands: vec![],
            agents: vec![],
            capabilities: vec![],
            permissions: PluginPermissions::default(),
            endpoint: None,
            signature: None,
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn load_from_file_succeeds() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("plugin.json");
        let mut f = fs::File::create(&path).unwrap();
        write!(f, "{}", minimal_manifest_json()).unwrap();

        let m = load(&path).unwrap();
        assert_eq!(m.id, "test-plugin");
    }

    #[test]
    fn load_returns_error_for_missing_file() {
        let result = load(Path::new("/nonexistent/plugin.json"));
        assert!(result.is_err());
    }

    #[test]
    fn scan_plugins_dir_finds_valid_plugins() {
        let root = TempDir::new().unwrap();
        // plugin subdir
        let plugin_dir = root.path().join("my-plugin");
        fs::create_dir(&plugin_dir).unwrap();
        let mut f = fs::File::create(plugin_dir.join("plugin.json")).unwrap();
        write!(f, "{}", minimal_manifest_json()).unwrap();
        // invalid subdir (no plugin.json)
        fs::create_dir(root.path().join("not-a-plugin")).unwrap();

        let found = scan_plugins_dir(root.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "test-plugin");
    }
}
