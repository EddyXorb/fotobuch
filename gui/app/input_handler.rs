use std::collections::HashSet;

use crate::state::{
    self, ActiveDrag, DataState, DragMode, DragSource, HoveredTarget, InteractionState,
    SlotSelection,
};

use super::pending::PendingCommand;

/// Top-level input dispatcher — called once per frame before painting.
pub fn handle(
    data: &mut DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
) -> HashSet<PendingCommand> {
    let mut cmds = HashSet::new();

    handle_timings_toggle(data, ctx);
    handle_drag_mode_toggle(interaction, ctx);
    handle_zoom(interaction, ctx);
    handle_undo_redo(ctx, &mut cmds);
    handle_config_panel_toggle(interaction, ctx);
    handle_place_hotkey(interaction, ctx, &mut cmds);

    let drag_action = handle_drag_complete(interaction, ctx, &mut cmds);
    handle_drag_start(interaction, ctx);

    if !drag_action {
        handle_escape(interaction, ctx);
        handle_select_all(data, interaction, ctx);
        handle_click(interaction, ctx);
    }

    cmds
}

fn handle_drag_mode_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M)) {
        interaction.drag.mode = interaction.drag.mode.toggle();
    }
}

fn handle_timings_toggle(data: &mut DataState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
        data.timings.show = !data.timings.show;
    }
}

fn handle_zoom(interaction: &mut InteractionState, ctx: &egui::Context) {
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
    let old_zoom = interaction.viewport.zoom;
    interaction.viewport.zoom = state::apply_zoom_delta(old_zoom, delta);
    let ratio = interaction.viewport.zoom / old_zoom;
    let cursor_y = ctx
        .pointer_hover_pos()
        .map_or(interaction.viewport.scroll.viewport_top, |p| p.y);
    let rel = cursor_y - interaction.viewport.scroll.viewport_top;
    interaction.viewport.scroll.pending_scroll_y =
        Some(interaction.viewport.scroll.scroll_y * ratio + rel * (ratio - 1.0));
}

fn handle_undo_redo(ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    let redo = ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
            || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
    });
    if redo {
        cmds.insert(PendingCommand::Redo);
        return;
    }

    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
        cmds.insert(PendingCommand::Undo);
    }
}

/// Detects the start of a drag from any source (slot, nav page, pool row).
fn handle_drag_start(interaction: &mut InteractionState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.secondary_pressed()) {
        return;
    }
    if !matches!(interaction.drag.active, ActiveDrag::Idle) {
        return;
    }
    let cursor = match ctx.pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };

    let drag_source = if let Some(HoveredTarget::Page {
        page,
        slot: Some(slot),
    }) = &interaction.hovered
    {
        Some(DragSource::Slot {
            src_page: *page,
            src_slot: *slot,
            cursor_at_drag_start: cursor,
        })
    } else if let Some(HoveredTarget::NavPage(nav_page)) = &interaction.hovered {
        Some(DragSource::NavPage {
            src_page: *nav_page,
            cursor_at_drag_start: cursor,
        })
    } else if let Some(HoveredTarget::PoolItem(pool_id)) = &interaction.hovered {
        let pool_id = pool_id.clone();
        let ids = if interaction.selections.photos.is_selected(&pool_id) {
            interaction.selections.photos.ids()
        } else {
            vec![pool_id]
        };
        Some(DragSource::Pool { photo_ids: ids })
    } else {
        None
    };
    if let Some(src) = drag_source {
        interaction.drag.active = ActiveDrag::Dragging(src);
    }
}

/// Handles RMB release for all drag sources. Returns `true` when a drag action was taken.
fn handle_drag_complete(
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) -> bool {
    if !ctx.input(|i| i.pointer.secondary_released()) {
        return false;
    }
    let source = match std::mem::replace(&mut interaction.drag.active, ActiveDrag::Idle) {
        ActiveDrag::Dragging(src) => src,
        ActiveDrag::Idle => return false,
    };
    match source {
        DragSource::Slot {
            src_page, src_slot, ..
        } => {
            complete_slot_drag(interaction, cmds, src_page, src_slot);
        }
        DragSource::NavPage { src_page, .. } => {
            complete_nav_drag(interaction, cmds, src_page);
        }
        DragSource::Pool { photo_ids } => {
            complete_pool_drag(interaction, cmds, photo_ids);
        }
    }
    true
}

fn complete_slot_drag(
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
) {
    let hovered_slot = interaction.hovered.as_ref().and_then(|h| h.slot());
    let effective_page = interaction.hovered.as_ref().and_then(|h| h.page_idx());
    match (hovered_slot, interaction.drag.mode) {
        (Some((dst_page, dst_slot)), DragMode::Swap) => {
            dispatch_swap(cmds, src_page, src_slot, dst_page, dst_slot);
        }
        (Some((dst_page, _)), DragMode::Move) => {
            dispatch_move(interaction, cmds, src_page, src_slot, dst_page);
        }
        (None, DragMode::Move) => {
            if let Some(dst_page) = effective_page {
                dispatch_move(interaction, cmds, src_page, src_slot, dst_page);
            }
        }
        (None, DragMode::Swap) => {}
    }
}

fn complete_nav_drag(
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
) {
    let dst_page = match interaction.hovered.as_ref().and_then(|h| h.as_nav_page()) {
        Some(p) if p != src_page => p,
        _ => return,
    };
    cmds.insert(PendingCommand::PageSwap {
        left: src_page,
        right: dst_page,
    });
}

fn complete_pool_drag(
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    photo_ids: Vec<String>,
) {
    if let Some(dst_page) = interaction.hovered.as_ref().and_then(|h| h.page_idx()) {
        cmds.insert(PendingCommand::Place {
            photo_ids,
            dst_page: Some(dst_page),
        });
    }
}

fn handle_escape(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        interaction.drag.active = ActiveDrag::Idle;
        interaction.selections.slots.clear();
    }
}

fn handle_select_all(data: &DataState, interaction: &mut InteractionState, ctx: &egui::Context) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
        return;
    }
    let current_page = interaction
        .hovered
        .as_ref()
        .and_then(|h| h.slot())
        .map(|(p, _)| p)
        .or(match &interaction.selections.slots {
            SlotSelection::OnPage { page, .. } => Some(*page),
            SlotSelection::None => None,
        });
    if let Some(page) = current_page {
        let slot_count = data
            .project
            .layout
            .get(page)
            .map(|lp| lp.slots.len())
            .unwrap_or(0);
        interaction.selections.slots.select_all_on(page, slot_count);
    }
}

fn handle_config_panel_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Comma)) {
        interaction.config.open = !interaction.config.open;
    }
}

fn handle_place_hotkey(
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P)) {
        return;
    }
    let ids = interaction.selections.photos.ids();
    if ids.is_empty() {
        return;
    }
    cmds.insert(PendingCommand::Place {
        photo_ids: ids,
        dst_page: interaction
            .hovered
            .as_ref()
            .and_then(HoveredTarget::central_page),
    });
}

/// Move: cross-page allowed. If the dragged slot is part of the current selection,
/// all selected slots on the source page are moved together.
fn dispatch_move(
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
) {
    let src_slots: Vec<usize> = if interaction.selections.slots.is_selected(src_page, src_slot) {
        match &interaction.selections.slots {
            SlotSelection::OnPage { page, slots, .. } if *page == src_page => {
                slots.iter().copied().collect()
            }
            _ => vec![src_slot],
        }
    } else {
        vec![src_slot]
    };

    cmds.insert(PendingCommand::Move {
        src_page,
        src_slots,
        dst_page,
    });
}

/// Swap: same-page only, single slot.
fn dispatch_swap(
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
) {
    if src_page == dst_page && src_slot == dst_slot {
        return;
    }
    cmds.insert(PendingCommand::Swap {
        src_page,
        src_slot,
        dst_page,
        dst_slot,
    });
}

fn handle_click(interaction: &mut InteractionState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.primary_clicked()) {
        return;
    }
    let modifiers = ctx.input(|i| i.modifiers);
    if let Some((page, slot)) = interaction.hovered.as_ref().and_then(|h| h.slot()) {
        if modifiers.shift {
            interaction.selections.slots.range_to(page, slot);
        } else if modifiers.ctrl || modifiers.command {
            interaction.selections.slots.toggle(page, slot);
        } else {
            interaction.selections.slots = SlotSelection::single(page, slot);
        }
    } else {
        interaction.selections.slots.clear();
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
            state.interaction.selections.slots = SlotSelection::OnPage {
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
        let mut cmds = HashSet::new();
        dispatch_move(&mut state.interaction, &mut cmds, 0, 2, 1);
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
        let mut cmds = HashSet::new();
        dispatch_move(&mut state.interaction, &mut cmds, 0, 3, 1);
        let PendingCommand::Move { src_slots, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*src_slots, vec![3]);
    }

    #[test]
    fn dispatch_swap_cross_page_emits_command() {
        let mut cmds = HashSet::new();
        dispatch_swap(&mut cmds, 0, 0, 1, 2);
        assert_eq!(cmds.len(), 1, "cross-page swap must emit a command");
    }

    #[test]
    fn dispatch_swap_ignores_same_slot() {
        let mut cmds = HashSet::new();
        dispatch_swap(&mut cmds, 0, 1, 0, 1);

        assert!(cmds.is_empty(), "same-slot swap must be ignored");
    }

    fn state_with_pool_selection(ids: Vec<&str>) -> GuiState {
        let mut state = GuiState::new(ProjectState::default());
        for id in &ids {
            state.interaction.selections.photos.toggle(id.to_string());
        }
        state
    }

    #[test]
    fn place_hotkey_emits_place_with_hovered_page() {
        let mut state = state_with_pool_selection(vec!["a.jpg"]);
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 2,
            slot: None,
        });
        let mut cmds = HashSet::new();
        cmds.insert(PendingCommand::Place {
            photo_ids: state.interaction.selections.photos.ids(),
            dst_page: state
                .interaction
                .hovered
                .as_ref()
                .and_then(HoveredTarget::central_page),
        });
        let PendingCommand::Place { dst_page, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*dst_page, Some(2));
    }

    #[test]
    fn place_hotkey_emits_place_without_target_when_no_hover() {
        let mut state = state_with_pool_selection(vec!["a.jpg"]);
        state.interaction.hovered = None;
        let mut cmds = HashSet::new();
        cmds.insert(PendingCommand::Place {
            photo_ids: state.interaction.selections.photos.ids(),
            dst_page: state
                .interaction
                .hovered
                .as_ref()
                .and_then(HoveredTarget::central_page),
        });
        let PendingCommand::Place { dst_page, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*dst_page, None);
    }

    #[test]
    fn place_hotkey_no_op_when_selection_empty() {
        let state = GuiState::new(ProjectState::default());
        let ids = state.interaction.selections.photos.ids();
        assert!(ids.is_empty());
    }

    #[test]
    fn pool_drag_complete_emits_place_on_hovered_page() {
        let mut state = GuiState::new(ProjectState::default());
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 1,
            slot: None,
        });
        let mut cmds = HashSet::new();
        complete_pool_drag(&mut state.interaction, &mut cmds, vec!["a.jpg".into()]);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::Place { dst_page, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*dst_page, Some(1));
    }

    #[test]
    fn pool_drag_complete_cancels_without_hovered_page() {
        let mut state = GuiState::new(ProjectState::default());
        state.interaction.hovered = None;
        let mut cmds = HashSet::new();
        complete_pool_drag(&mut state.interaction, &mut cmds, vec!["a.jpg".into()]);
        assert!(cmds.is_empty(), "no hovered page → no command");
    }

    #[test]
    fn nav_drag_complete_emits_page_swap() {
        let mut state = GuiState::new(ProjectState::default());
        state.interaction.hovered = Some(HoveredTarget::NavPage(2));
        let mut cmds = HashSet::new();
        complete_nav_drag(&mut state.interaction, &mut cmds, 0);
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
        state.interaction.hovered = Some(HoveredTarget::NavPage(1));
        let mut cmds = HashSet::new();
        complete_nav_drag(&mut state.interaction, &mut cmds, 1);
        assert!(cmds.is_empty(), "same page → no-op");
    }
}
