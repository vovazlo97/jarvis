use std::collections::HashMap;

use once_cell::sync::Lazy;
use parking_lot::RwLock;

/// A registered automation agent provided by a plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentEntry {
    /// Unique agent identifier (e.g. "browser-agent").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Capability tags this agent advertises (e.g. ["web", "automation"]).
    pub capabilities: Vec<String>,
    /// ID of the plugin that registered this agent.
    pub plugin_id: String,
}

/// Global agent registry — thread-safe, append/remove at runtime.
static REGISTRY: Lazy<RwLock<HashMap<String, AgentEntry>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register an agent. Returns an error if an agent with the same ID is already registered.
pub fn register(entry: AgentEntry) -> Result<(), String> {
    let mut guard = REGISTRY.write();
    if guard.contains_key(&entry.id) {
        return Err(format!(
            "Agent '{}' is already registered (plugin: {})",
            entry.id, guard[&entry.id].plugin_id
        ));
    }
    guard.insert(entry.id.clone(), entry);
    Ok(())
}

/// Unregister an agent by ID. No-op if not registered.
pub fn unregister(id: &str) {
    REGISTRY.write().remove(id);
}

/// Look up an agent by ID.
pub fn get(id: &str) -> Option<AgentEntry> {
    REGISTRY.read().get(id).cloned()
}

/// Return all registered agents sorted by ID for deterministic ordering.
pub fn list_all() -> Vec<AgentEntry> {
    let mut entries: Vec<AgentEntry> = REGISTRY.read().values().cloned().collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// Remove all agents registered by a specific plugin (called on plugin unload).
pub fn unregister_plugin(plugin_id: &str) {
    REGISTRY.write().retain(|_, v| v.plugin_id != plugin_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, plugin_id: &str) -> AgentEntry {
        AgentEntry {
            id: id.to_string(),
            name: format!("{id} agent"),
            capabilities: vec!["test".to_string()],
            plugin_id: plugin_id.to_string(),
        }
    }

    fn cleanup(ids: &[&str], plugin_ids: &[&str]) {
        for id in ids {
            unregister(id);
        }
        for pid in plugin_ids {
            unregister_plugin(pid);
        }
    }

    #[test]
    fn register_and_get_agent() {
        let id = "test-reg-get";
        register(make_entry(id, "plugin-a")).unwrap();
        let found = get(id).unwrap();
        assert_eq!(found.id, id);
        cleanup(&[id], &[]);
    }

    #[test]
    fn register_duplicate_returns_error() {
        let id = "test-reg-dup";
        register(make_entry(id, "plugin-a")).unwrap();
        let result = register(make_entry(id, "plugin-b"));
        assert!(result.is_err());
        cleanup(&[id], &[]);
    }

    #[test]
    fn unregister_removes_agent() {
        let id = "test-unreg";
        register(make_entry(id, "plugin-a")).unwrap();
        unregister(id);
        assert!(get(id).is_none());
    }

    #[test]
    fn list_all_returns_sorted_entries() {
        cleanup(&[], &["plugin-list-test"]);
        register(make_entry("zz-agent", "plugin-list-test")).unwrap();
        register(make_entry("aa-agent", "plugin-list-test")).unwrap();
        let all = list_all();
        let positions: Vec<usize> = ["aa-agent", "zz-agent"]
            .iter()
            .filter_map(|id| all.iter().position(|e| &e.id == id))
            .collect();
        assert!(
            positions[0] < positions[1],
            "list_all should be sorted by id"
        );
        cleanup(&[], &["plugin-list-test"]);
    }

    #[test]
    fn unregister_plugin_removes_all_its_agents() {
        register(make_entry("agent-x1", "plugin-x")).unwrap();
        register(make_entry("agent-x2", "plugin-x")).unwrap();
        unregister_plugin("plugin-x");
        assert!(get("agent-x1").is_none());
        assert!(get("agent-x2").is_none());
    }

    #[test]
    fn get_returns_none_for_unknown_agent() {
        assert!(get("definitely-not-registered-xyz").is_none());
    }
}
