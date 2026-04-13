use crate::state::{self, DragState, GuiState, Selection};

use super::pending::PendingCommand;

/// Top-level input dispatcher — called once per frame before painting.
///
/// Returns commands that require background execution.
pub fn handle(state: &mut GuiState, ctx: &egui::Context) -> Vec<PendingCommand> {
    let mut cmds = Vec::new();

    handle_timings_toggle(state, ctx);
    handle_zoom(state, ctx);
    handle_undo_redo(state, ctx, &mut cmds);

    let drag_action = handle_drag_complete(state, ctx, &mut cmds);
    handle_drag_start(state, ctx);

    if !drag_action {
        handle_escape(state, ctx);
        handle_select_all(state, ctx);
        handle_click(state, ctx);
    }

    cmds
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

fn handle_undo_redo(state: &mut GuiState, ctx: &egui::Context, cmds: &mut Vec<PendingCommand>) {
    let redo = ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
            || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
    });
    if redo {
        mark_all_dirty(state);
        cmds.push(PendingCommand::Redo);
        return;
    }

    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
        mark_all_dirty(state);
        cmds.push(PendingCommand::Undo);
    }
}

fn mark_all_dirty(state: &mut GuiState) {
    for d in &mut state.page_dirty {
        *d = true;
    }
}

/// Checks whether a drag-and-drop gesture has completed.
///
/// Returns `true` when a drag action was taken (command emitted or drag cancelled
/// on a different target), so that click handling can be suppressed.
fn handle_drag_complete(
    state: &mut GuiState,
    ctx: &egui::Context,
    cmds: &mut Vec<PendingCommand>,
) -> bool {
    if !ctx.input(|i| i.pointer.primary_released()) {
        return false;
    }

    let (src_page, src_slot, is_move) = match state.drag {
        DragState::Dragging {
            src_page,
            src_slot,
            is_move,
        } => (src_page, src_slot, is_move),
        DragState::Idle => return false,
    };

    state.drag = DragState::Idle;

    let (dst_page, dst_slot) = match state.hovered_slot {
        Some(s) => s,
        None => return true, // drag cancelled — no target
    };

    // Phase 3: same-page only. Cross-page drops are silently ignored.
    if dst_page != src_page {
        return true; // drag landed on a different page → suppress click
    }
    if dst_slot == src_slot {
        return false; // same slot → treat as normal click
    }

    // Mark affected pages dirty immediately for instant visual feedback.
    if let Some(d) = state.page_dirty.get_mut(src_page) {
        *d = true;
    }

    if is_move {
        cmds.push(PendingCommand::Move {
            src_page,
            src_slot,
            dst_page,
        });
    } else {
        cmds.push(PendingCommand::Swap {
            src_page,
            src_slot,
            dst_page,
            dst_slot,
        });
    }

    true
}

fn handle_drag_start(state: &mut GuiState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.primary_pressed()) {
        return;
    }
    if !matches!(state.drag, DragState::Idle) {
        return;
    }
    if let Some((page, slot)) = state.hovered_slot {
        let is_move = ctx.input(|i| i.key_down(egui::Key::M));
        state.drag = DragState::Dragging {
            src_page: page,
            src_slot: slot,
            is_move,
        };
    }
}

fn handle_escape(state: &mut GuiState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        state.drag = DragState::Idle;
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
