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
    if !ctx.input(|i| i.pointer.secondary_released()) {
        return false;
    }

    let (src_page, src_slot, is_move) = match state.drag {
        DragState::Dragging {
            src_page,
            src_slot,
            is_move,
            cursor_at_drag_start: _,
        } => (src_page, src_slot, is_move),
        DragState::Idle => return false,
    };

    state.drag = DragState::Idle;

    let (dst_page, dst_slot) = match state.hovered_slot {
        Some(s) => s,
        None => return true, // drag cancelled — no target
    };

    if is_move {
        dispatch_move(state, cmds, src_page, src_slot, dst_page);
    } else {
        dispatch_swap(state, cmds, src_page, src_slot, dst_page, dst_slot);
    }

    true
}

fn handle_drag_start(state: &mut GuiState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.secondary_pressed()) {
        return;
    }
    if !matches!(state.drag, DragState::Idle) {
        return;
    }
    let cursor = match ctx.pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };
    if let Some((page, slot)) = state.hovered_slot {
        let is_move = ctx.input(|i| i.key_down(egui::Key::M));
        state.drag = DragState::Dragging {
            src_page: page,
            src_slot: slot,
            is_move,
            cursor_at_drag_start: cursor,
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

/// Move: cross-page allowed. If the dragged slot is part of the current selection,
/// all selected slots on the source page are moved together.
fn dispatch_move(
    state: &mut GuiState,
    cmds: &mut Vec<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
) {
    let src_slots: Vec<usize> = if state.selection.is_selected(src_page, src_slot) {
        match &state.selection {
            Selection::OnPage { page, slots, .. } if *page == src_page => {
                slots.iter().copied().collect()
            }
            _ => vec![src_slot],
        }
    } else {
        vec![src_slot]
    };

    for &p in &[src_page, dst_page] {
        if let Some(d) = state.page_dirty.get_mut(p) {
            *d = true;
        }
    }
    cmds.push(PendingCommand::Move {
        src_page,
        src_slots,
        dst_page,
    });
}

/// Swap: same-page only, single slot.
fn dispatch_swap(
    state: &mut GuiState,
    cmds: &mut Vec<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
) {
    if src_page == dst_page && src_slot == dst_slot {
        return; // same slot → no-op
    }
    for &p in &[src_page, dst_page] {
        if let Some(d) = state.page_dirty.get_mut(p) {
            *d = true;
        }
    }
    cmds.push(PendingCommand::Swap {
        src_page,
        src_slot,
        dst_page,
        dst_slot,
    });
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use fotobuch::dto_models::ProjectState;

    use super::*;
    use crate::state::GuiState;

    fn state_with_selection(sel_page: usize, sel_slots: Vec<usize>) -> GuiState {
        let mut state = GuiState::new(ProjectState::default());
        if !sel_slots.is_empty() {
            state.selection = Selection::OnPage {
                page: sel_page,
                slots: BTreeSet::from_iter(sel_slots),
                anchor: 0,
            };
        }
        state
    }

    #[test]
    fn dispatch_move_uses_selection_when_dragged_slot_is_selected() {
        let mut state = state_with_selection(0, vec![1, 2, 3]);
        state.page_dirty = vec![false, false];
        let mut cmds = Vec::new();
        dispatch_move(&mut state, &mut cmds, 0, 2, 1);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::Move {
            src_page,
            src_slots,
            dst_page,
        } = &cmds[0]
        else {
            panic!("expected Move");
        };
        assert_eq!(*src_page, 0);
        assert_eq!(*dst_page, 1);
        assert_eq!(*src_slots, vec![1, 2, 3]);
    }

    #[test]
    fn dispatch_move_uses_single_slot_when_not_in_selection() {
        let mut state = state_with_selection(0, vec![0, 1]);
        state.page_dirty = vec![false, false];
        let mut cmds = Vec::new();
        dispatch_move(&mut state, &mut cmds, 0, 3, 1); // slot 3 not in selection
        let PendingCommand::Move { src_slots, .. } = &cmds[0] else {
            panic!()
        };
        assert_eq!(*src_slots, vec![3]);
    }

    #[test]
    fn dispatch_swap_cross_page_emits_command() {
        let mut state = GuiState::new(ProjectState::default());
        state.page_dirty = vec![false, false];
        let mut cmds = Vec::new();
        dispatch_swap(&mut state, &mut cmds, 0, 0, 1, 2);
        assert_eq!(cmds.len(), 1, "cross-page swap must emit a command");
        assert!(state.page_dirty[0] && state.page_dirty[1]);
    }

    #[test]
    fn dispatch_swap_ignores_same_slot() {
        let mut state = GuiState::new(ProjectState::default());
        state.page_dirty = vec![false];
        let mut cmds = Vec::new();
        dispatch_swap(&mut state, &mut cmds, 0, 1, 0, 1);
        assert!(cmds.is_empty(), "same-slot swap must be ignored");
    }
}
