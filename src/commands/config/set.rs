//! `config set` — Konfigurationswert per Dot-Notation setzen.

use std::path::Path;

use anyhow::{Result, anyhow};
use serde_yaml::Value;

use crate::{
    commands::{CommandOutput, run_write_command},
    models::ProjectConfig,
    state_manager::{ReadOnlyState, WriteConfigState},
};

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

    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_config_state();
        let old_value = set_config_key(&mut view, key, value)?;

        Ok((
            format!("config set {key}: {value}"),
            ConfigSetResult {
                key: key.to_string(),
                old_value,
                new_value: value.to_string(),
            },
        ))
    })
}

/// Set `key` (dot-notation) to `value` on the config view, returning the old value.
///
/// Optional fields that are currently unset (and therefore absent from the
/// serialized config, e.g. `book.cover.spine_text`) can be set as well; a key
/// is only accepted if it survives a deserialize/serialize round trip.
/// If the auto-detected type (number, bool) is rejected, the raw string is
/// tried, so e.g. `book.title 2024` stores the string `"2024"`.
fn set_config_key(s: &mut WriteConfigState, key: &str, value: &str) -> Result<String> {
    let config_value =
        serde_yaml::to_value(s.config()).map_err(|e| anyhow!("Failed to serialize config: {e}"))?;

    let parts: Vec<&str> = key.split('.').collect();
    let (head, last) = parts.split_at(parts.len() - 1);
    let last_key = last[0];

    let parent =
        lookup(&config_value, head).ok_or_else(|| anyhow!("Unknown config key: '{key}'"))?;
    if !parent.is_mapping() {
        return Err(anyhow!("Cannot set '{key}': parent is not a mapping"));
    }

    // Absent keys are unset optional fields or typos; the round trip decides.
    let old_value = parent
        .get(last_key)
        .map_or_else(|| "null".to_string(), value_to_string);

    let typed = parse_yaml_value(value);
    let new_config = match apply_value(&config_value, &parts, typed.clone()) {
        Err(e) if !typed.is_string() => {
            apply_value(&config_value, &parts, Value::String(value.to_string())).map_err(|_| e)
        }
        other => other,
    }
    .map_err(|e| anyhow!("Cannot set '{key}' to '{value}': {e}"))?;

    // serde silently drops unknown fields — reject keys that did not survive.
    let round_trip = serde_yaml::to_value(&new_config)
        .map_err(|e| anyhow!("Failed to serialize config: {e}"))?;
    if lookup(&round_trip, &parts).is_none() {
        return Err(anyhow!("Unknown config key: '{key}'"));
    }

    *s.config_mut() = new_config;
    Ok(old_value)
}

/// Insert `new` at the dot-path `parts` into a copy of `root` and deserialize it.
fn apply_value(root: &Value, parts: &[&str], new: Value) -> Result<ProjectConfig> {
    let mut root = root.clone();
    let (head, last) = parts.split_at(parts.len() - 1);
    let mut current = &mut root;
    for part in head {
        current = current
            .get_mut(*part)
            .ok_or_else(|| anyhow!("missing parent '{part}'"))?;
    }
    current
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("parent is not a mapping"))?
        .insert(Value::String(last[0].to_string()), new);
    Ok(serde_yaml::from_value(root)?)
}

/// Follow the dot-path `parts` in `root`.
fn lookup<'a>(root: &'a Value, parts: &[&str]) -> Option<&'a Value> {
    parts.iter().try_fold(root, |v, p| v.get(*p))
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
    use crate::state_manager::StateManager;

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
    fn test_set_title_numeric_string() {
        let (_tmp, root) = open_tmp();
        let res = config_set(&root, "book.title", "2024").unwrap();
        assert_eq!(res.result.new_value, "2024");

        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(mgr.state.config.book.title, "2024");
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_set_unset_spine_text_numeric_string() {
        let (_tmp, root) = open_tmp();
        let res = config_set(&root, "book.cover.spine_text", "2024").unwrap();
        assert_eq!(res.result.old_value, "null");

        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(
            mgr.state.config.book.cover.spine_text.as_deref(),
            Some("2024")
        );
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_overwrite_spine_text_reports_old_value() {
        let (_tmp, root) = open_tmp();
        config_set(&root, "book.cover.spine_text", "Urlaub").unwrap();
        let res = config_set(&root, "book.cover.spine_text", "Sommer 2024").unwrap();
        assert_eq!(res.result.old_value, "Urlaub");

        let mgr = StateManager::open(&root).unwrap();
        assert_eq!(
            mgr.state.config.book.cover.spine_text.as_deref(),
            Some("Sommer 2024")
        );
        mgr.finish("noop").unwrap();
    }

    #[test]
    fn test_unknown_nested_key_errors() {
        let (_tmp, root) = open_tmp();
        let err = config_set(&root, "book.cover.spine_txt", "x").unwrap_err();
        assert!(err.to_string().contains("Unknown config key"));
    }

    #[test]
    fn test_numeric_field_still_rejects_text() {
        let (_tmp, root) = open_tmp();
        let err = config_set(&root, "book.gap_mm", "true").unwrap_err();
        assert!(err.to_string().contains("Cannot set"));
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
