use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::{DragMode, GuiState};

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) -> HashSet<PendingCommand> {
    egui::Panel::top("toolbar")
        .show_inside(ui, |ui| show(ui, &mut state.drag_mode))
        .inner
}

fn show(ui: &mut egui::Ui, drag_mode: &mut DragMode) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();

    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("Build"));
        ui.add_enabled(false, egui::Button::new("Release"));
        if ui.add(egui::Button::new("↩")).clicked() {
            cmds.insert(PendingCommand::Undo);
        }
        if ui.add(egui::Button::new("↪")).clicked() {
            cmds.insert(PendingCommand::Redo);
        }
        ui.add_enabled(false, egui::Button::new("⚙"));

        ui.separator();

        if ui
            .add(egui::Button::selectable(
                *drag_mode == DragMode::Swap,
                "⇄ Swap",
            ))
            .clicked()
        {
            *drag_mode = DragMode::Swap;
        }
        if ui
            .add(egui::Button::selectable(
                *drag_mode == DragMode::Move,
                "→ Move",
            ))
            .clicked()
        {
            *drag_mode = DragMode::Move;
        }
    });

    cmds
}
