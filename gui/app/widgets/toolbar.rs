use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::{DragMode, HoveredTarget, InteractionState};

pub fn draw(ui: &mut egui::Ui, interaction: &mut InteractionState) -> HashSet<PendingCommand> {
    egui::Panel::top("toolbar")
        .show_inside(ui, |ui| show(ui, interaction))
        .inner
}

fn show(ui: &mut egui::Ui, interaction: &mut InteractionState) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();
    ui.horizontal(|ui| {
        build_buttons(ui);
        history_buttons(ui, &mut cmds);
        config_button(ui, interaction);
        ui.separator();
        place_button(ui, interaction, &mut cmds);
        ui.separator();
        drag_mode_buttons(ui, interaction);
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

fn config_button(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .add(egui::Button::selectable(
            interaction.config.open,
            "⚙ Config",
        ))
        .clicked()
    {
        interaction.config.open = !interaction.config.open;
    }
}

fn place_button(
    ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
) {
    let place_enabled = !interaction.selections.photos.is_empty();
    if ui
        .add_enabled(place_enabled, egui::Button::new("Place"))
        .clicked()
    {
        cmds.insert(PendingCommand::Place {
            photo_ids: interaction.selections.photos.ids(),
            dst_page: interaction
                .hovered
                .as_ref()
                .and_then(HoveredTarget::central_page),
        });
    }
}

fn drag_mode_buttons(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .add(egui::Button::selectable(
            interaction.drag.mode == DragMode::Swap,
            "⇄ Swap",
        ))
        .clicked()
    {
        interaction.drag.mode = DragMode::Swap;
    }
    if ui
        .add(egui::Button::selectable(
            interaction.drag.mode == DragMode::Move,
            "→ Move",
        ))
        .clicked()
    {
        interaction.drag.mode = DragMode::Move;
    }
}
