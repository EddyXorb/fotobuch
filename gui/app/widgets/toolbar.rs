use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::{DragMode, GuiState, HoveredTarget};

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) -> HashSet<PendingCommand> {
    egui::Panel::top("toolbar")
        .show_inside(ui, |ui| show(ui, state))
        .inner
}

fn show(ui: &mut egui::Ui, state: &mut GuiState) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();
    ui.horizontal(|ui| {
        build_buttons(ui);
        history_buttons(ui, &mut cmds);
        config_button(ui, state);
        ui.separator();
        place_button(ui, state, &mut cmds);
        ui.separator();
        drag_mode_buttons(ui, state);
    });
    cmds
}

fn build_buttons(ui: &mut egui::Ui) {
    ui.add_enabled(false, egui::Button::new("Build"));
    ui.add_enabled(false, egui::Button::new("Release"));
}

fn history_buttons(ui: &mut egui::Ui, cmds: &mut HashSet<PendingCommand>) {
    if ui.add(egui::Button::new("↩")).clicked() {
        cmds.insert(PendingCommand::Undo);
    }
    if ui.add(egui::Button::new("↪")).clicked() {
        cmds.insert(PendingCommand::Redo);
    }
}

fn config_button(ui: &mut egui::Ui, state: &mut GuiState) {
    if ui
        .add(egui::Button::selectable(
            state.config_panel.open,
            "⚙ Config",
        ))
        .clicked()
    {
        state.config_panel.open = !state.config_panel.open;
    }
}

fn place_button(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    let place_enabled = !state.selections.photos.is_empty();
    if ui
        .add_enabled(place_enabled, egui::Button::new("Place"))
        .clicked()
    {
        for d in &mut state.cache.dirty {
            *d = true;
        }
        cmds.insert(PendingCommand::Place {
            photo_ids: state.selections.photos.ids(),
            dst_page: state.hovered.as_ref().and_then(HoveredTarget::central_page),
        });
    }
}

fn drag_mode_buttons(ui: &mut egui::Ui, state: &mut GuiState) {
    if ui
        .add(egui::Button::selectable(
            state.drag.mode == DragMode::Swap,
            "⇄ Swap",
        ))
        .clicked()
    {
        state.drag.mode = DragMode::Swap;
    }
    if ui
        .add(egui::Button::selectable(
            state.drag.mode == DragMode::Move,
            "→ Move",
        ))
        .clicked()
    {
        state.drag.mode = DragMode::Move;
    }
}
