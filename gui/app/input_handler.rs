mod drag;
mod hotkeys;

use std::collections::HashSet;

use crate::state::{DataState, InteractionState};

use super::pending::PendingCommand;

/// Top-level input dispatcher — called once per frame before painting.
pub fn handle(
    data: &mut DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();

    hotkeys::handle_timings_toggle(data, ctx);
    hotkeys::handle_drag_mode_toggle(interaction, ctx);
    hotkeys::handle_zoom(interaction, ctx);
    hotkeys::handle_undo_redo(ctx, &mut cmds);
    hotkeys::handle_config_panel_toggle(interaction, ctx);
    hotkeys::handle_place_hotkey(data, interaction, ctx, &mut cmds);
    hotkeys::handle_delete(data, interaction, ctx, &mut cmds);
    hotkeys::handle_rebuild(data, interaction, ctx, &mut cmds);
    hotkeys::handle_goto_toggle(interaction, ctx);
    hotkeys::handle_home_end(interaction, data, ctx);
    hotkeys::handle_fit_width(interaction, ctx);
    hotkeys::handle_release_build(ctx, &mut cmds);
    hotkeys::handle_add_hotkey(interaction, ctx);

    let drag_action = drag::handle_drag_complete(data, interaction, ctx, &mut cmds);
    drag::handle_drag_start(interaction, ctx);

    if !drag_action {
        hotkeys::handle_escape(interaction, ctx);
        hotkeys::handle_select_all(data, interaction, ctx);
        hotkeys::handle_click(interaction, ctx);
    }

    cmds
}

#[cfg(test)]
mod tests;
