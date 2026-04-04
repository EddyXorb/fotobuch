//! `config set` — Konfigurationswert per Dot-Notation setzen.

use std::path::Path;

use anyhow::{Result, anyhow};
use serde_yaml::Value;

use crate::{commands::CommandOutput, dto_models::ProjectConfig, state_manager::StateManager};

/// Result of a successful `config set` call.
#[derive(Debug, Clone)]
pub struct ConfigSetResult {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

/// Set a config value by dot-notation key (e.g. `"book.dpi"`).
///
/// # Errors
/// - Empty or invalid key
/// - Key not found in config hierarchy
/// - Value cannot be deserialized into the expected type
pub fn config_set(
    project_root: &Path,
    key: &str,
    value: &str,
) -> Result<CommandOutput<ConfigSetResult>> {
    if key.is_empty() || key.split('.').any(|p| p.is_empty()) {
        return Err(anyhow!("Invalid config key: '{key}'"));
    }

    let mut mgr = StateManager::open(project_root)?;

    let mut config_value = serde_yaml::to_value(&mgr.state.config)
        .map_err(|e| anyhow!("Failed to serialize config: {e}"))?;

    let parts: Vec<&str> = key.split('.').collect();
    let (head, last_key) = parts.split_at(parts.len() - 1);

    // Navigate to the parent mapping
    let mut current = &mut config_value;
    for part in head {
        current = current
            .get_mut(*part)
            .ok_or_else(|| anyhow!("Unknown config key: '{key}'"))?;
    }

    let last_key = last_key[0];

    // Remember old value
    let old_value = current
        .get(last_key)
        .map(value_to_string)
        .ok_or_else(|| anyhow!("Unknown config key: '{key}'"))?;

    // Set new value
    let new_yaml = parse_yaml_value(value);
    match current {
        Value::Mapping(map) => {
            map.insert(Value::String(last_key.to_string()), new_yaml);
        }
        _ => return Err(anyhow!("Cannot set '{key}': parent is not a mapping")),
    }

    // Deserialize back — validates types and enum variants
    let new_config: ProjectConfig = serde_yaml::from_value(config_value)
        .map_err(|e| anyhow!("Cannot set '{key}' to '{value}': {e}"))?;

    mgr.state.config = new_config;
    let state = mgr.finish(&format!("config set {key}: {value}"))?;

    Ok(CommandOutput {
        result: ConfigSetResult {
            key: key.to_string(),
            old_value,
            new_value: value.to_string(),
        },
        state,
    })
}

/// Auto-detect the YAML type from a string value.
fn parse_yaml_value(s: &str) -> Value {
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }
    if let Ok(i) = s.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Number(f.into());
    }
    Value::String(s.to_string())
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        _ => serde_yaml::to_string(v)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::page::test_fixtures::{make_state_with_layout, setup_repo};

    fn open_tmp() -> (tempfile::TempDir, std::path::PathBuf) {
        let state = make_state_with_layout(vec![vec![]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);
        let path = tmp.path().to_path_buf();
        (tmp, path)
    }

    #[test]
    fn test_set_dpi() {
        let (_tmp, root) = open_tmp();
        let res = config_set(&root, "book.dpi", "150").unwrap();
        assert_eq!(res.result.key, "book.dpi");
        assert_eq!(res.result.new_value, "150");

        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(mgr.state.config.book.dpi, 150.0);
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_set_gap_mm_float() {
        let (_tmp, root) = open_tmp();
        let res = config_set(&root, "book.gap_mm", "3.5").unwrap();
        assert_eq!(res.result.new_value, "3.5");

        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(mgr.state.config.book.gap_mm, 3.5);
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_set_nested_bool() {
        let (_tmp, root) = open_tmp();
        config_set(&root, "book.cover.active", "true").unwrap();

        let mgr = StateManager::open(&root).unwrap();
        assert!(mgr.state.config.book.cover.active);
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_set_title_string() {
        let (_tmp, root) = open_tmp();
        let res = config_set(&root, "book.title", "Mein Buch").unwrap();
        assert_eq!(res.result.new_value, "Mein Buch");

        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(mgr.state.config.book.title, "Mein Buch");
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_set_cover_mode_string() {
        let (_tmp, root) = open_tmp();
        // Valid enum variant — serde must accept it
        config_set(&root, "book.cover.mode", "spread").unwrap();
    }

    #[test]
    fn test_unknown_key_errors() {
        let (_tmp, root) = open_tmp();
        let err = config_set(&root, "book.nonexistent", "1").unwrap_err();
        assert!(err.to_string().contains("Unknown config key"));
    }

    #[test]
    fn test_invalid_value_errors() {
        let (_tmp, root) = open_tmp();
        let err = config_set(&root, "book.dpi", "abc").unwrap_err();
        assert!(err.to_string().contains("Cannot set"));
    }

    #[test]
    fn test_empty_key_errors() {
        let (_tmp, root) = open_tmp();
        assert!(config_set(&root, "", "1").is_err());
        assert!(config_set(&root, "book..dpi", "1").is_err());
    }

    #[test]
    fn test_roundtrip() {
        let (_tmp, root) = open_tmp();
        config_set(&root, "book.dpi", "72").unwrap();
        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(mgr.state.config.book.dpi, 72.0);
        mgr.finish("noop").unwrap();
    }
}
