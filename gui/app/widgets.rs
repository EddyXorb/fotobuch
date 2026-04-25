use std::collections::HashSet;

use crate::state::{DataState, InteractionState};

pub mod timings_panel;

mod add_dialog;
mod central_panel;
mod config_window;
mod geometry;
mod goto_dialog;
mod page_nav;
mod photo_pool;
mod rebuild_confirm;
mod statusbar;
mod toolbar;

pub fn draw_widgets(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    data: &DataState,
    interaction: &mut InteractionState,
) -> HashSet<super::pending::PendingCommand> {
    // Clear per-frame hover state before widgets re-populate it.
    interaction.hovered = None;
    let mut cmds = toolbar::draw(ui, interaction);
    statusbar::draw(ui, data, interaction);
    // Side panels must come before the central panel (egui ordering requirement).
    photo_pool::draw(ui, data, interaction);
    page_nav::draw(ui, data, interaction, &mut cmds);
    central_panel::draw(ui, data, interaction);

    if interaction.config.open {
        config_window::show(ctx, data, interaction, &mut cmds);
    }

    let num_pages = data.project.layout.len();
    goto_dialog::show(ctx, interaction, num_pages);
    rebuild_confirm::show(ctx, interaction, &mut cmds);
    add_dialog::show(ctx, interaction);

    cmds
}
