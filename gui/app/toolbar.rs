use std::collections::HashSet;

use crate::app::pending::PendingCommand;

/// Toolbar with disabled action stubs (Phase 2 — no commands yet).
pub fn show(ui: &mut egui::Ui) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();

    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("Build"));
        ui.add_enabled(false, egui::Button::new("Release"));
        if (ui.add(egui::Button::new("↩")).clicked()) {
            cmds.insert(PendingCommand::Undo);
        }
        if (ui.add(egui::Button::new("↪")).clicked()) {
            cmds.insert(PendingCommand::Redo);
        }
        ui.add_enabled(false, egui::Button::new("⚙"));
    });

    cmds
}
