use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::GuiState;

pub const NAV_THUMB_MAX_EDGE_PX: u32 = 120;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState, _cmds: &mut HashSet<PendingCommand>) {
    egui::SidePanel::right("page_nav")
        .resizable(true)
        .min_width(100.0)
        .max_width(200.0)
        .default_width(120.0)
        .show_inside(ui, |ui| {
            ui.label("Page Nav");
            let _ = state;
        });
}
