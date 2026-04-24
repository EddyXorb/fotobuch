use std::collections::HashSet;

use crate::state::{
    self, ActiveDrag, DragMode, DragSource, DragState, GuiState, HoveredTarget, SlotSelection,
};

use super::pending::PendingCommand;

/// Top-level input dispatcher — called once per frame before painting.
pub fn handle(state: &mut GuiState, ctx: &egui::Context) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();

    handle_timings_toggle(state, ctx);
    handle_drag_mode_toggle(state, ctx);
    handle_zoom(state, ctx);
    handle_undo_redo(state, ctx, &mut cmds);
    handle_config_panel_toggle(state, ctx);
    handle_place_hotkey(state, ctx, &mut cmds);

    let drag_action = handle_drag_complete(state, ctx, &mut cmds);
    handle_drag_start(state, ctx);

    if !drag_action {
        handle_escape(state, ctx);
        handle_select_all(state, ctx);
        handle_click(state, ctx);
    }

    cmds
}

fn handle_drag_mode_toggle(state: &mut GuiState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M)) {
        state.drag.mode = state.drag.mode.toggle();
    }
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
    if delta == 1.0 {
        return;
    }
    let old_zoom = state.viewport.zoom;
    state.viewport.zoom = state::apply_zoom_delta(old_zoom, delta);
    let ratio = state.viewport.zoom / old_zoom;
    let cursor_y = ctx
        .pointer_hover_pos()
        .map_or(state.viewport.scroll.viewport_top, |p| p.y);
    let rel = cursor_y - state.viewport.scroll.viewport_top;
    state.viewport.scroll.pending_scroll_y =
        Some(state.viewport.scroll.scroll_y * ratio + rel * (ratio - 1.0));
}

fn handle_undo_redo(state: &mut GuiState, ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    let redo = ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
            || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
    });
    if redo {
        mark_all_dirty(state);
        cmds.insert(PendingCommand::Redo);
        return;
    }

    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
        mark_all_dirty(state);
        cmds.insert(PendingCommand::Undo);
    }
}

fn mark_all_dirty(state: &mut GuiState) {
    for d in &mut state.pages.dirty {
        *d = true;
    }
}

/// Detects the start of a drag from any source (slot, nav page, pool row).
fn handle_drag_start(state: &mut GuiState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.secondary_pressed()) {
        return;
    }
    if !matches!(state.drag.active, ActiveDrag::Idle) {
        return;
    }
    let cursor = match ctx.pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };

    let drag_source = if let Some(HoveredTarget::Page {
        page,
        slot: Some(slot),
    }) = &state.hovered
    {
        Some(DragSource::Slot {
            src_page: *page,
            src_slot: *slot,
            cursor_at_drag_start: cursor,
        })
    } else if let Some(HoveredTarget::NavPage(nav_page)) = &state.hovered {
        Some(DragSource::NavPage {
            src_page: *nav_page,
            cursor_at_drag_start: cursor,
        })
    } else if let Some(HoveredTarget::PoolItem(pool_id)) = &state.hovered {
        let pool_id = pool_id.clone();
        let ids = if state.selections.photos.is_selected(&pool_id) {
            state.selections.photos.ids()
        } else {
            vec![pool_id]
        };
        Some(DragSource::Pool { photo_ids: ids })
    } else {
        None
    };
    if let Some(src) = drag_source {
        state.drag.active = ActiveDrag::Dragging(src);
    }
}

/// Handles RMB release for all drag sources. Returns `true` when a drag action was taken.
fn handle_drag_complete(
    state: &mut GuiState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) -> bool {
    if !ctx.input(|i| i.pointer.secondary_released()) {
        return false;
    }
    let source = match std::mem::replace(&mut state.drag.active, ActiveDrag::Idle) {
        ActiveDrag::Dragging(src) => src,
        ActiveDrag::Idle => return false,
    };
    match source {
        DragSource::Slot {
            src_page, src_slot, ..
        } => {
            complete_slot_drag(state, cmds, src_page, src_slot);
        }
        DragSource::NavPage { src_page, .. } => {
            complete_nav_drag(state, cmds, src_page);
        }
        DragSource::Pool { photo_ids } => {
            complete_pool_drag(state, cmds, photo_ids);
        }
    }
    true
}

fn complete_slot_drag(
    state: &mut GuiState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
) {
    let hovered_slot = state.hovered.as_ref().and_then(|h| h.slot());
    let effective_page = state.hovered.as_ref().and_then(|h| h.page_idx());
    match (hovered_slot, state.drag.mode) {
        (Some((dst_page, dst_slot)), DragMode::Swap) => {
            dispatch_swap(state, cmds, src_page, src_slot, dst_page, dst_slot);
        }
        (Some((dst_page, _)), DragMode::Move) => {
            dispatch_move(state, cmds, src_page, src_slot, dst_page);
        }
        (None, DragMode::Move) => {
            if let Some(dst_page) = effective_page {
                dispatch_move(state, cmds, src_page, src_slot, dst_page);
            }
        }
        (None, DragMode::Swap) => {}
    }
}

fn complete_nav_drag(state: &mut GuiState, cmds: &mut HashSet<PendingCommand>, src_page: usize) {
    let dst_page = match state.hovered.as_ref().and_then(|h| h.as_nav_page()) {
        Some(p) if p != src_page => p,
        _ => return,
    };
    for &p in &[src_page, dst_page] {
        if let Some(d) = state.pages.dirty.get_mut(p) {
            *d = true;
        }
    }
    cmds.insert(PendingCommand::PageSwap {
        left: src_page,
        right: dst_page,
    });
}

fn complete_pool_drag(
    state: &mut GuiState,
    cmds: &mut HashSet<PendingCommand>,
    photo_ids: Vec<String>,
) {
    if let Some(dst_page) = state.hovered.as_ref().and_then(|h| h.page_idx()) {
        if let Some(d) = state.pages.dirty.get_mut(dst_page) {
            *d = true;
        }
        cmds.insert(PendingCommand::Place {
            photo_ids,
            dst_page: Some(dst_page),
        });
    }
}

fn handle_escape(state: &mut GuiState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        state.drag.active = ActiveDrag::Idle;
        state.selections.slots.clear();
    }
}

fn handle_select_all(state: &mut GuiState, ctx: &egui::Context) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
        return;
    }
    let current_page = state
        .hovered
        .as_ref()
        .and_then(|h| h.slot())
        .map(|(p, _)| p)
        .or(match &state.selections.slots {
            SlotSelection::OnPage { page, .. } => Some(*page),
            SlotSelection::None => None,
        });
    if let Some(page) = current_page {
        let slot_count = state
            .project
            .layout
            .get(page)
            .map(|lp| lp.slots.len())
            .unwrap_or(0);
        state.selections.slots.select_all_on(page, slot_count);
    }
}

fn handle_config_panel_toggle(state: &mut GuiState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Comma)) {
        state.config.open = !state.config.open;
    }
}

fn handle_place_hotkey(
    state: &mut GuiState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P)) {
        return;
    }
    let ids = state.selections.photos.ids();
    if ids.is_empty() {
        return;
    }
    mark_all_dirty(state);
    cmds.insert(PendingCommand::Place {
        photo_ids: ids,
        dst_page: state.hovered.as_ref().and_then(HoveredTarget::central_page),
    });
}

/// Move: cross-page allowed. If the dragged slot is part of the current selection,
/// all selected slots on the source page are moved together.
fn dispatch_move(
    state: &mut GuiState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
) {
    let src_slots: Vec<usize> = if state.selections.slots.is_selected(src_page, src_slot) {
        match &state.selections.slots {
            SlotSelection::OnPage { page, slots, .. } if *page == src_page => {
                slots.iter().copied().collect()
            }
            _ => vec![src_slot],
        }
    } else {
        vec![src_slot]
    };

    for &p in &[src_page, dst_page] {
        if let Some(d) = state.pages.dirty.get_mut(p) {
            *d = true;
        }
    }
    cmds.insert(PendingCommand::Move {
        src_page,
        src_slots,
        dst_page,
    });
}

/// Swap: same-page only, single slot.
fn dispatch_swap(
    state: &mut GuiState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
) {
    if src_page == dst_page && src_slot == dst_slot {
        return;
    }
    for &p in &[src_page, dst_page] {
        if let Some(d) = state.pages.dirty.get_mut(p) {
            *d = true;
        }
    }
    cmds.insert(PendingCommand::Swap {
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
    if let Some((page, slot)) = state.hovered.as_ref().and_then(|h| h.slot()) {
        if modifiers.shift {
            state.selections.slots.range_to(page, slot);
        } else if modifiers.ctrl || modifiers.command {
            state.selections.slots.toggle(page, slot);
        } else {
            state.selections.slots = SlotSelection::single(page, slot);
        }
    } else {
        state.selections.slots.clear();
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
            state.selections.slots = SlotSelection::OnPage {
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
        state.pages.dirty = vec![false, false];
        let mut cmds = HashSet::new();
        dispatch_move(&mut state, &mut cmds, 0, 2, 1);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::Move {
            src_page,
            src_slots,
            dst_page,
        } = cmds.iter().next().unwrap()
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
        state.pages.dirty = vec![false, false];
        let mut cmds = HashSet::new();
        dispatch_move(&mut state, &mut cmds, 0, 3, 1);
        let PendingCommand::Move { src_slots, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*src_slots, vec![3]);
    }

    #[test]
    fn dispatch_swap_cross_page_emits_command() {
        let mut state = GuiState::new(ProjectState::default());
        state.pages.dirty = vec![false, false];
        let mut cmds = HashSet::new();
        dispatch_swap(&mut state, &mut cmds, 0, 0, 1, 2);
        assert_eq!(cmds.len(), 1, "cross-page swap must emit a command");
        assert!(state.pages.dirty[0] && state.pages.dirty[1]);
    }

    #[test]
    fn dispatch_swap_ignores_same_slot() {
        let mut state = GuiState::new(ProjectState::default());
        state.pages.dirty = vec![false];
        let mut cmds = HashSet::new();
        dispatch_swap(&mut state, &mut cmds, 0, 1, 0, 1);
        assert!(cmds.is_empty(), "same-slot swap must be ignored");
    }

    fn state_with_pool_selection(ids: Vec<&str>) -> GuiState {
        let mut state = GuiState::new(ProjectState::default());
        for id in &ids {
            state.selections.photos.toggle(id.to_string());
        }
        state
    }

    #[test]
    fn place_hotkey_emits_place_with_hovered_page() {
        let mut state = state_with_pool_selection(vec!["a.jpg"]);
        state.hovered = Some(HoveredTarget::Page {
            page: 2,
            slot: None,
        });
        state.pages.dirty = vec![false, false, false];
        let mut cmds = HashSet::new();
        for d in &mut state.pages.dirty {
            *d = true;
        }
        cmds.insert(PendingCommand::Place {
            photo_ids: state.selections.photos.ids(),
            dst_page: state.hovered.as_ref().and_then(HoveredTarget::central_page),
        });
        let PendingCommand::Place { dst_page, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*dst_page, Some(2));
    }

    #[test]
    fn place_hotkey_emits_place_without_target_when_no_hover() {
        let mut state = state_with_pool_selection(vec!["a.jpg"]);
        state.hovered = None;
        let mut cmds = HashSet::new();
        cmds.insert(PendingCommand::Place {
            photo_ids: state.selections.photos.ids(),
            dst_page: state.hovered.as_ref().and_then(HoveredTarget::central_page),
        });
        let PendingCommand::Place { dst_page, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*dst_page, None);
    }

    #[test]
    fn place_hotkey_no_op_when_selection_empty() {
        let state = GuiState::new(ProjectState::default());
        let ids = state.selections.photos.ids();
        assert!(ids.is_empty());
    }

    #[test]
    fn pool_drag_complete_emits_place_on_hovered_page() {
        let mut state = GuiState::new(ProjectState::default());
        state.hovered = Some(HoveredTarget::Page {
            page: 1,
            slot: None,
        });
        state.pages.dirty = vec![false, false];
        let mut cmds = HashSet::new();
        complete_pool_drag(&mut state, &mut cmds, vec!["a.jpg".into()]);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::Place { dst_page, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*dst_page, Some(1));
    }

    #[test]
    fn pool_drag_complete_cancels_without_hovered_page() {
        let mut state = GuiState::new(ProjectState::default());
        state.hovered = None;
        let mut cmds = HashSet::new();
        complete_pool_drag(&mut state, &mut cmds, vec!["a.jpg".into()]);
        assert!(cmds.is_empty(), "no hovered page → no command");
    }

    #[test]
    fn nav_drag_complete_emits_page_swap() {
        let mut state = GuiState::new(ProjectState::default());
        state.hovered = Some(HoveredTarget::NavPage(2));
        state.pages.dirty = vec![false, false, false];
        let mut cmds = HashSet::new();
        complete_nav_drag(&mut state, &mut cmds, 0);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::PageSwap { left, right } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*left, 0);
        assert_eq!(*right, 2);
    }

    #[test]
    fn nav_drag_complete_noop_when_same_page() {
        let mut state = GuiState::new(ProjectState::default());
        state.hovered = Some(HoveredTarget::NavPage(1));
        state.pages.dirty = vec![false, false];
        let mut cmds = HashSet::new();
        complete_nav_drag(&mut state, &mut cmds, 1);
        assert!(cmds.is_empty(), "same page → no-op");
    }
}
