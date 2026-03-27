//! `fotobuch config` command - Show current configuration

use anyhow::Result;
use serde_yaml::Value;
use std::path::Path;

use crate::dto_models::ProjectConfig;
use crate::state_manager::StateManager;

/// Configuration result with resolved values and raw YAML for default detection
#[derive(Debug, Clone)]
pub struct ConfigResult {
    /// Fully resolved configuration with all defaults filled in
    pub resolved: ProjectConfig,
    /// Raw YAML value of the config section for detecting which fields were explicitly set
    pub raw: Value,
}

/// Show current configuration (read-only)
///
/// Loads the project state and returns both the resolved configuration (with defaults)
/// and the raw YAML value for detecting which fields were explicitly set vs. defaulted.
///
/// StateManager::open() auto-commits any pending user edits before reading config.
pub fn config(project_root: &Path) -> Result<ConfigResult> {
    let mgr = StateManager::open(project_root)?;

    Ok(ConfigResult {
        resolved: mgr.state.config.clone(),
        raw: mgr.raw_config().clone(),
    })
}

/// Renders configuration as annotated YAML with "# default" comments
///
/// Traverses the resolved config tree and marks fields not present in the raw YAML
/// with "# default" annotation. The output is valid YAML.
pub fn render_config(result: &ConfigResult) -> Result<String> {
    let mut output = String::new();
    render_annotated(
        &to_yaml_value(&result.resolved)?,
        &result.raw,
        0,
        &mut output,
    );
    Ok(output)
}

/// Convert ProjectConfig to a serde_yaml::Value for rendering
fn to_yaml_value(config: &ProjectConfig) -> Result<Value> {
    serde_yaml::to_value(config).map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))
}

/// Recursively renders a YAML value tree with annotations for defaulted fields
fn render_annotated(resolved: &Value, raw: &Value, indent: usize, output: &mut String) {
    match resolved {
        Value::Mapping(map) => {
            let raw_map = raw.as_mapping();
            for (key, value) in map {
                let key_str = key.as_str().unwrap_or("?");

                // Check if this key was present in the raw YAML
                let is_default = raw_map.map(|m| !m.contains_key(key)).unwrap_or(true);

                if value.is_mapping() {
                    // Nested mapping: recurse without annotation on the key itself
                    write_indent(output, indent);
                    output.push_str(&format!("{key_str}:\n"));

                    let child_raw = raw_map
                        .and_then(|m| m.get(key))
                        .cloned()
                        .unwrap_or(Value::Mapping(Default::default()));

                    render_annotated(value, &child_raw, indent + 2, output);
                } else if value.is_sequence() {
                    // Sequence: render inline or multiline
                    write_indent(output, indent);
                    output.push_str(&format!("{key_str}: {:<24}", format_sequence(value)));

                    if is_default {
                        output.push_str("# default");
                    }
                    output.push('\n');
                } else {
                    // Scalar: render with possible annotation
                    write_indent(output, indent);
                    let val_str = format_scalar(value);
                    if is_default {
                        output.push_str(&format!("{key_str}: {:<24}# default\n", val_str));
                    } else {
                        output.push_str(&format!("{key_str}: {val_str}\n"));
                    }
                }
            }
        }
        _ => {
            // Scalar at top level (unlikely for config)
            write_indent(output, indent);
            output.push_str(&format!("{}\n", format_scalar(resolved)));
        }
    }
}

/// Format a scalar value for YAML output
fn format_scalar(value: &Value) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(_) => {
            // Delegate to serde_yaml so strings with special chars (e.g. `{`, `[`)
            // are quoted correctly and produce valid YAML.
            serde_yaml::to_string(value)
                .unwrap_or_default()
                .trim()
                .to_string()
        }
        Value::Null => "null".to_string(),
        _ => serde_yaml::to_string(value)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// Format a sequence value for YAML output
fn format_sequence(value: &Value) -> String {
    match value {
        Value::Sequence(seq) => {
            let items: Vec<String> = seq.iter().map(format_scalar).collect();
            format!("[{}]", items.join(", "))
        }
        _ => format_scalar(value),
    }
}

/// Write indentation to output
fn write_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_scalar_bool() {
        let val = Value::Bool(true);
        assert_eq!(format_scalar(&val), "true");
    }

    #[test]
    fn test_format_scalar_number() {
        let val = Value::Number(42.into());
        assert_eq!(format_scalar(&val), "42");
    }

    #[test]
    fn test_format_scalar_string() {
        let val = Value::String("hello".to_string());
        assert_eq!(format_scalar(&val), "hello");
    }

    #[test]
    fn test_format_scalar_null() {
        let val = Value::Null;
        assert_eq!(format_scalar(&val), "null");
    }

    #[test]
    fn test_format_sequence() {
        let items = vec![
            Value::Number(1.into()),
            Value::Number(2.into()),
            Value::Number(3.into()),
        ];
        let val = Value::Sequence(items);
        assert_eq!(format_sequence(&val), "[1, 2, 3]");
    }

    #[test]
    fn test_render_annotated_defaults() {
        let resolved: Value =
            serde_yaml::from_str("key1: value1\nkey2: 42\nnested:\n  inner: true").unwrap();

        let raw: Value = serde_yaml::from_str("key1: value1").unwrap();

        let mut output = String::new();
        render_annotated(&resolved, &raw, 0, &mut output);

        // Check that key2 and nested are marked as default
        assert!(output.contains("key2:") && output.contains("# default"));
        assert!(output.contains("nested:"));
        assert!(output.contains("inner:") && output.contains("# default"));
    }

    #[test]
    fn test_render_annotated_no_defaults() {
        let resolved: Value = serde_yaml::from_str("key1: value1").unwrap();
        let raw: Value = serde_yaml::from_str("key1: value1").unwrap();

        let mut output = String::new();
        render_annotated(&resolved, &raw, 0, &mut output);

        // key1 should not have default annotation
        assert!(output.contains("key1: value1") && !output.contains("# default"));
    }
}
