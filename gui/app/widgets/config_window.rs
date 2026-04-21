use std::collections::HashSet;

use serde_yaml::Value;

use crate::app::pending::PendingCommand;
use crate::state::GuiState;

pub fn show(ctx: &egui::Context, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    let config_value = match serde_yaml::to_value(&state.project_state.config) {
        Ok(v) => v,
        Err(_) => return,
    };

    egui::Window::new("Config")
        .default_size([380.0, 520.0])
        .resizable(true)
        .open(&mut state.config_panel.open)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                walk(
                    ui,
                    &mut state.config_panel.edit_buffers,
                    cmds,
                    "",
                    &config_value,
                );
            });
        });
}

fn walk(
    ui: &mut egui::Ui,
    buffers: &mut std::collections::HashMap<String, String>,
    cmds: &mut HashSet<PendingCommand>,
    path: &str,
    value: &Value,
) {
    match value {
        Value::Mapping(m) => {
            for (k, v) in m {
                let key = k.as_str().unwrap_or("?");
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                if v.is_mapping() {
                    egui::CollapsingHeader::new(key)
                        .default_open(true)
                        .show(ui, |ui| walk(ui, buffers, cmds, &child_path, v));
                } else {
                    ui.horizontal(|ui| {
                        ui.label(key);
                        leaf_widget(ui, buffers, cmds, &child_path, v);
                    });
                }
            }
        }
        Value::Sequence(_) => {
            ui.label("<list — read-only>");
        }
        _ => {}
    }
}

fn leaf_widget(
    ui: &mut egui::Ui,
    buffers: &mut std::collections::HashMap<String, String>,
    cmds: &mut HashSet<PendingCommand>,
    key: &str,
    current: &Value,
) {
    let current_str = match current {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => return,
    };

    // Bool: render as checkbox, commit immediately on change.
    if let Value::Bool(b) = current {
        let mut val = *b;
        if ui.checkbox(&mut val, "").changed() {
            cmds.insert(PendingCommand::ConfigSet {
                key: key.to_string(),
                value: val.to_string(),
            });
        }
        return;
    }

    let buf = buffers
        .entry(key.to_string())
        .or_insert_with(|| current_str.clone());

    let resp = ui.text_edit_singleline(buf);

    if resp.lost_focus() {
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        if !escape && *buf != current_str {
            cmds.insert(PendingCommand::ConfigSet {
                key: key.to_string(),
                value: buf.clone(),
            });
        }
        buffers.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use serde_yaml::Value;

    use crate::app::pending::PendingCommand;

    fn walk_collect_keys(value: &Value) -> Vec<String> {
        let mut keys = Vec::new();
        collect_keys("", value, &mut keys);
        keys
    }

    fn collect_keys(path: &str, value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Mapping(m) => {
                for (k, v) in m {
                    let key = k.as_str().unwrap_or("?");
                    let child = if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{path}.{key}")
                    };
                    if v.is_mapping() {
                        collect_keys(&child, v, out);
                    } else {
                        out.push(child);
                    }
                }
            }
            _ => out.push(path.to_string()),
        }
    }

    #[test]
    fn walk_flat_keys() {
        let yaml: Value = serde_yaml::from_str("a: 1\nb: two\nc: true").unwrap();
        let keys = walk_collect_keys(&yaml);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    #[test]
    fn walk_nested_keys_use_dot_notation() {
        let yaml: Value = serde_yaml::from_str("outer:\n  inner: 42").unwrap();
        let keys = walk_collect_keys(&yaml);
        assert!(keys.contains(&"outer.inner".to_string()));
        assert!(!keys.contains(&"outer".to_string()));
    }

    fn simulate_leaf_commit(
        current_str: &str,
        buf_str: &str,
        escape: bool,
    ) -> HashSet<PendingCommand> {
        let key = "book.dpi";
        let mut buffers: HashMap<String, String> = HashMap::new();
        buffers.insert(key.to_string(), buf_str.to_string());

        let mut cmds = HashSet::new();

        // Simulate lost_focus logic from leaf_widget
        if !escape && buf_str != current_str {
            cmds.insert(PendingCommand::ConfigSet {
                key: key.to_string(),
                value: buf_str.to_string(),
            });
        }
        buffers.remove(key);

        cmds
    }

    #[test]
    fn commit_only_on_change() {
        // Same value → no command
        let cmds = simulate_leaf_commit("150", "150", false);
        assert!(cmds.is_empty(), "no change → no command");

        // Different value → command
        let cmds = simulate_leaf_commit("150", "300", false);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::ConfigSet { key, value } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(key, "book.dpi");
        assert_eq!(value, "300");
    }

    #[test]
    fn escape_discards_buffer() {
        let cmds = simulate_leaf_commit("150", "999", true);
        assert!(cmds.is_empty(), "escape → buffer discarded, no command");
    }
}
