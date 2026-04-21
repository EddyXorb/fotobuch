use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::GuiState;

pub fn draw(ui: &mut egui::Ui, _state: &mut GuiState, _cmds: &mut HashSet<PendingCommand>) {
    egui::SidePanel::left("photo_pool")
        .resizable(true)
        .min_width(220.0)
        .max_width(400.0)
        .default_width(260.0)
        .show_inside(ui, |ui| {
            ui.label("Photo Pool");
        });
}
