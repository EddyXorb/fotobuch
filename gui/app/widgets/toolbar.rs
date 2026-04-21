use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::GuiState;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) -> HashSet<PendingCommand> {
    egui::Panel::top("toolbar")
        .show_inside(ui, |ui| show(ui, state))
        .inner
}

fn show(ui: &mut egui::Ui, state: &mut GuiState) -> HashSet<PendingCommand> {
    use crate::state::DragMode;
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

        // Config toggle button
        let config_selected = state.config_panel.open;
        if ui
            .add(egui::Button::selectable(config_selected, "⚙ Config"))
            .clicked()
        {
            state.config_panel.open = !state.config_panel.open;
        }

        ui.separator();

        // Place button — active when pool selection is non-empty
        let place_enabled = !state.pool_selection.is_empty();
        if ui
            .add_enabled(place_enabled, egui::Button::new("Place"))
            .clicked()
        {
            for d in &mut state.page_dirty {
                *d = true;
            }
            cmds.insert(PendingCommand::Place {
                photo_ids: state.pool_selection.ids(),
                dst_page: state.hovered_page,
            });
        }

        ui.separator();

        if ui
            .add(egui::Button::selectable(
                state.drag_mode == DragMode::Swap,
                "⇄ Swap",
            ))
            .clicked()
        {
            state.drag_mode = DragMode::Swap;
        }
        if ui
            .add(egui::Button::selectable(
                state.drag_mode == DragMode::Move,
                "→ Move",
            ))
            .clicked()
        {
            state.drag_mode = DragMode::Move;
        }
    });

    cmds
}
