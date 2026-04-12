use crate::state::{self, GuiState, Selection};

/// Top-level input dispatcher — called once per frame before painting.
pub fn handle(state: &mut GuiState, ctx: &egui::Context) {
    handle_timings_toggle(state, ctx);
    handle_zoom(state, ctx);
    handle_escape(state, ctx);
    handle_select_all(state, ctx);
    handle_click(state, ctx);
}

fn handle_timings_toggle(state: &mut GuiState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
        state.timings.show = !state.timings.show;
    }
}

fn handle_zoom(state: &mut GuiState, ctx: &egui::Context) {
    let delta = ctx.input(|i| {
        if i.modifiers.ctrl {
            i.zoom_delta()
        } else {
            1.0
        }
    });
    if delta != 1.0 {
        state.zoom = state::apply_zoom_delta(state.zoom, delta);
    }
}

fn handle_escape(state: &mut GuiState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        state.selection.clear();
    }
}

fn handle_select_all(state: &mut GuiState, ctx: &egui::Context) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
        return;
    }
    let current_page = state
        .hovered_slot
        .map(|(p, _)| p)
        .or(match &state.selection {
            Selection::OnPage { page, .. } => Some(*page),
            Selection::None => None,
        });
    if let Some(page) = current_page {
        let slot_count = state
            .project_state
            .layout
            .get(page)
            .map(|lp| lp.slots.len())
            .unwrap_or(0);
        state.selection.select_all_on(page, slot_count);
    }
}

fn handle_click(state: &mut GuiState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.primary_clicked()) {
        return;
    }
    let modifiers = ctx.input(|i| i.modifiers);
    if let Some((page, slot)) = state.hovered_slot {
        if modifiers.shift {
            state.selection.range_to(page, slot);
        } else if modifiers.ctrl || modifiers.command {
            state.selection.toggle(page, slot);
        } else {
            state.selection = Selection::single(page, slot);
        }
    } else {
        state.selection.clear();
    }
}
