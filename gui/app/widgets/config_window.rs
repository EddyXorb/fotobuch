use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::GuiState;

pub fn show(_ctx: &egui::Context, state: &mut GuiState, _cmds: &mut HashSet<PendingCommand>) {
    egui::Window::new("Config")
        .default_size([380.0, 520.0])
        .resizable(true)
        .open(&mut state.config_panel.open)
        .show(_ctx, |_ui| {});
}
