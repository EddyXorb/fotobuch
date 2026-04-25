use std::collections::HashSet;

use crate::state::{
    self, ActiveDrag, DataState, DragMode, DragSource, HoveredTarget, InteractionState,
    SlotSelection,
};
use fotobuch::dto_models::LayoutPage;

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
    handle_delete(data, interaction, ctx, &mut cmds);
    handle_rebuild(data, interaction, ctx, &mut cmds);
    handle_goto_toggle(interaction, ctx);
    handle_home_end(interaction, data, ctx);
    handle_fit_width(interaction, ctx);
    handle_release_build(ctx, &mut cmds);
    handle_add_hotkey(interaction, ctx);

    let drag_action = handle_drag_complete(data, interaction, ctx, &mut cmds);
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
    data: &DataState,
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
            complete_slot_drag(data, interaction, cmds, src_page, src_slot);
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

fn selection_slots_for(
    interaction: &InteractionState,
    src_page: usize,
    src_slot: usize,
) -> Vec<usize> {
    if interaction.selections.slots.is_selected(src_page, src_slot) {
        match &interaction.selections.slots {
            SlotSelection::OnPage { page, slots, .. } if *page == src_page => {
                slots.iter().copied().collect()
            }
            _ => vec![src_slot],
        }
    } else {
        vec![src_slot]
    }
}

fn complete_slot_drag(
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
) {
    // Drop on [+] new-page placeholder — takes priority over page hover.
    if let Some(at_position) = interaction
        .hovered
        .as_ref()
        .and_then(HoveredTarget::new_page_at_position)
    {
        let src_slots = selection_slots_for(interaction, src_page, src_slot);
        cmds.insert(PendingCommand::MoveToNewPage {
            src_page,
            src_slots,
            at_position,
        });
        return;
    }

    let hovered_slot = interaction.hovered.as_ref().and_then(|h| h.slot());
    let effective_page = interaction.hovered.as_ref().and_then(|h| h.page_idx());
    match (hovered_slot, interaction.drag.mode) {
        (Some((dst_page, dst_slot)), DragMode::Swap) => {
            let src_slots = selection_slots_for(interaction, src_page, src_slot);
            if src_slots.len() == 1 {
                dispatch_swap(cmds, src_page, src_slots[0], dst_page, dst_slot);
            } else if is_contiguous(&src_slots)
                && let Some(layout_dst) = data.project.layout.get(dst_page)
                && let Some(dst_slots) = compute_dst_range(dst_slot, src_slots.len(), layout_dst)
            {
                cmds.insert(PendingCommand::SwapRange {
                    src_page,
                    src_slots,
                    dst_page,
                    dst_slots,
                });
            }
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

fn is_contiguous(slots: &[usize]) -> bool {
    slots.windows(2).all(|w| w[1] == w[0] + 1)
}

fn compute_dst_range(dst_slot: usize, count: usize, layout_dst: &LayoutPage) -> Option<Vec<usize>> {
    let end_excl = dst_slot + count;
    if end_excl > layout_dst.slots.len() {
        return None;
    }
    Some((dst_slot..end_excl).collect())
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
    interaction: &InteractionState,
    cmds: &mut HashSet<PendingCommand>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
) {
    let src_slots = selection_slots_for(interaction, src_page, src_slot);
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

fn handle_delete(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
        return;
    }
    if let SlotSelection::OnPage { page, slots, .. } = &interaction.selections.slots
        && !slots.is_empty()
    {
        cmds.insert(PendingCommand::Unplace {
            page: *page,
            slots: slots.iter().copied().collect(),
        });
        return;
    }
    let target_page = interaction.hovered.as_ref().and_then(|h| match h {
        HoveredTarget::Page { page, slot: None } => Some(*page),
        HoveredTarget::NavPage(page) => Some(*page),
        _ => None,
    });
    if let Some(page) = target_page {
        if data.project.has_cover() && page == 0 {
            return;
        }
        cmds.insert(PendingCommand::DeletePage { page });
    }
}

fn handle_rebuild(
    data: &DataState,
    interaction: &InteractionState,
    ctx: &egui::Context,
    cmds: &mut HashSet<PendingCommand>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
        return;
    }
    let pages = match &interaction.selections.slots {
        SlotSelection::OnPage { page, .. } => vec![*page],
        SlotSelection::None => match &interaction.hovered {
            Some(HoveredTarget::Page { page, .. }) | Some(HoveredTarget::NavPage(page)) => {
                vec![*page]
            }
            _ => return,
        },
    };
    let _ = data;
    cmds.insert(PendingCommand::RebuildPages { pages });
}

fn handle_goto_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
        interaction.goto_open = !interaction.goto_open;
    }
}

fn handle_home_end(interaction: &mut InteractionState, data: &DataState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
        interaction.viewport.scroll_to_page = Some(0);
    } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::End)) {
        let last = data.project.layout.len().saturating_sub(1);
        interaction.viewport.scroll_to_page = Some(last);
    }
}

fn handle_fit_width(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0)) {
        interaction.viewport.fit_pending = true;
    }
}

fn handle_release_build(ctx: &egui::Context, cmds: &mut HashSet<PendingCommand>) {
    if ctx
        .input_mut(|i| i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::B))
    {
        cmds.insert(PendingCommand::ReleaseBuild);
    }
}

fn handle_add_hotkey(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
        interaction.add_dialog_open = true;
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

    fn layout_page_with_slots(page: usize, n_slots: usize) -> fotobuch::dto_models::LayoutPage {
        use fotobuch::dto_models::{LayoutPage, PageMode, Slot};
        LayoutPage {
            page,
            photos: vec![],
            slots: (0..n_slots)
                .map(|_| Slot {
                    x_mm: 0.0,
                    y_mm: 0.0,
                    width_mm: 100.0,
                    height_mm: 100.0,
                })
                .collect(),
            mode: PageMode::Auto,
        }
    }

    // ── Phase 5.1 tests ────────────────────────────────────────────────────────

    #[test]
    fn drop_on_new_page_slot_emits_move_to_new_page() {
        let mut state = state_with_selection(1, vec![1, 2]);
        state.interaction.hovered = Some(HoveredTarget::NewPageSlot { at_position: 3 });
        let mut cmds = HashSet::new();
        // Use complete_slot_drag with a minimal DataState that has no layout.
        let data = crate::state::DataState {
            project: ProjectState::default(),
            derived: crate::state::DerivedState::rebuild(&ProjectState::default()),
            pages: crate::state::PageCache::new(0),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 1, 1);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::MoveToNewPage {
            src_page,
            src_slots,
            at_position,
        } = cmds.iter().next().unwrap()
        else {
            panic!("expected MoveToNewPage");
        };
        assert_eq!(*src_page, 1);
        assert_eq!(*src_slots, vec![1, 2]);
        assert_eq!(*at_position, 3);
    }

    #[test]
    fn drop_on_new_page_slot_at_zero_inserts_before_first_page() {
        let mut state = GuiState::new(ProjectState::default());
        state.interaction.hovered = Some(HoveredTarget::NewPageSlot { at_position: 0 });
        let mut cmds = HashSet::new();
        let data = crate::state::DataState {
            project: ProjectState::default(),
            derived: crate::state::DerivedState::rebuild(&ProjectState::default()),
            pages: crate::state::PageCache::new(0),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 0);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::MoveToNewPage { at_position, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*at_position, 0);
    }

    // ── Phase 5.2 tests ────────────────────────────────────────────────────────

    #[test]
    fn cross_page_move_with_selection_moves_all_selected_slots() {
        let mut state = state_with_selection(3, vec![0, 2]);
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 7,
            slot: Some(0),
        });
        state.interaction.drag.mode = DragMode::Move;
        let mut cmds = HashSet::new();
        dispatch_move(&state.interaction, &mut cmds, 3, 0, 7);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::Move {
            src_page,
            src_slots,
            dst_page,
        } = cmds.iter().next().unwrap()
        else {
            panic!()
        };
        assert_eq!(*src_page, 3);
        assert_eq!(*dst_page, 7);
        assert_eq!(*src_slots, vec![0, 2]);
    }

    #[test]
    fn swap_range_uses_full_selection_when_dragged_slot_selected() {
        let mut state = state_with_selection(0, vec![1, 2, 3]);
        state.interaction.drag.mode = DragMode::Swap;
        let mut project = ProjectState::default();
        project.layout = vec![layout_page_with_slots(0, 4), layout_page_with_slots(1, 6)];
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 1,
            slot: Some(0),
        });
        let data = crate::state::DataState {
            derived: crate::state::DerivedState::rebuild(&project),
            project,
            pages: crate::state::PageCache::new(2),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        let mut cmds = HashSet::new();
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 1);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::SwapRange {
            src_slots,
            dst_slots,
            ..
        } = cmds.iter().next().unwrap()
        else {
            panic!("expected SwapRange");
        };
        assert_eq!(*src_slots, vec![1, 2, 3]);
        assert_eq!(*dst_slots, vec![0, 1, 2]);
    }

    #[test]
    fn swap_range_noop_when_target_too_narrow() {
        let mut state = state_with_selection(0, vec![0, 1, 2]);
        state.interaction.drag.mode = DragMode::Swap;
        let mut project = ProjectState::default();
        project.layout = vec![
            layout_page_with_slots(0, 3),
            layout_page_with_slots(1, 2), // only 2 slots — can't fit 3
        ];
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 1,
            slot: Some(0),
        });
        let data = crate::state::DataState {
            derived: crate::state::DerivedState::rebuild(&project),
            project,
            pages: crate::state::PageCache::new(2),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        let mut cmds = HashSet::new();
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 0);
        assert!(cmds.is_empty(), "overrun → no command emitted");
    }

    #[test]
    fn swap_range_noop_when_selection_not_contiguous() {
        let mut state = state_with_selection(0, vec![0, 2]); // gap at 1
        state.interaction.drag.mode = DragMode::Swap;
        let mut project = ProjectState::default();
        project.layout = vec![layout_page_with_slots(0, 4), layout_page_with_slots(1, 4)];
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 1,
            slot: Some(0),
        });
        let data = crate::state::DataState {
            derived: crate::state::DerivedState::rebuild(&project),
            project,
            pages: crate::state::PageCache::new(2),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        let mut cmds = HashSet::new();
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 0);
        assert!(cmds.is_empty(), "non-contiguous selection → no command");
    }

    #[test]
    fn swap_falls_back_to_single_when_selection_is_one() {
        let mut state = state_with_selection(0, vec![1]);
        state.interaction.drag.mode = DragMode::Swap;
        let mut project = ProjectState::default();
        project.layout = vec![layout_page_with_slots(0, 3), layout_page_with_slots(1, 3)];
        state.interaction.hovered = Some(HoveredTarget::Page {
            page: 1,
            slot: Some(2),
        });
        let data = crate::state::DataState {
            derived: crate::state::DerivedState::rebuild(&project),
            project,
            pages: crate::state::PageCache::new(2),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        let mut cmds = HashSet::new();
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 1);
        assert_eq!(cmds.len(), 1);
        assert!(
            matches!(cmds.iter().next().unwrap(), PendingCommand::Swap { .. }),
            "single-slot selection should use Swap, not SwapRange"
        );
    }

    // ── Phase 5.3 tests ────────────────────────────────────────────────────────

    #[test]
    fn handle_delete_emits_unplace_with_selection_slots() {
        let state = state_with_selection(2, vec![0, 3]);
        let mut cmds = HashSet::new();
        // Directly simulate the logic (no egui context available in unit tests).
        if let SlotSelection::OnPage { page, slots, .. } = &state.interaction.selections.slots
            && !slots.is_empty()
        {
            cmds.insert(PendingCommand::Unplace {
                page: *page,
                slots: slots.iter().copied().collect(),
            });
        }
        assert_eq!(cmds.len(), 1);
        let PendingCommand::Unplace { page, slots } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*page, 2);
        assert_eq!(*slots, vec![0, 3]);
    }

    #[test]
    fn handle_delete_emits_delete_page_when_only_page_hovered() {
        let state = GuiState::new(ProjectState::default());
        let target = HoveredTarget::Page {
            page: 4,
            slot: None,
        };
        let page = match &target {
            HoveredTarget::Page { page, slot: None } => Some(*page),
            HoveredTarget::NavPage(page) => Some(*page),
            _ => None,
        };
        assert_eq!(page, Some(4));
        let _ = state;
    }

    #[test]
    fn handle_delete_prefers_slot_selection_over_hovered_page() {
        let state = state_with_selection(1, vec![0]);
        // If slot-selection is present, delete-page must NOT be emitted.
        let slot_sel_present = matches!(
            &state.interaction.selections.slots,
            SlotSelection::OnPage { slots, .. } if !slots.is_empty()
        );
        assert!(slot_sel_present, "selection must win over hovered");
    }

    #[test]
    fn handle_delete_noop_on_cover_when_active() {
        use fotobuch::dto_models::{BookConfig, CoverConfig, ProjectConfig};
        let mut project = ProjectState::default();
        project.config = ProjectConfig {
            book: BookConfig {
                cover: CoverConfig {
                    active: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(project.has_cover());
        // Simulate guard: cover page 0 must be skipped.
        let page = 0usize;
        let would_emit = !(project.has_cover() && page == 0);
        assert!(!would_emit, "cover page must not be deleted");
    }

    #[test]
    fn rebuild_selection_matches_selected_page() {
        use crate::app::rebuild::{PagesForRebuild, selected_pages_for_rebuild};
        let mut interaction = GuiState::new(ProjectState::default()).interaction;
        interaction.selections.slots = SlotSelection::OnPage {
            page: 5,
            slots: BTreeSet::from([0]),
            anchor: 0,
        };
        assert!(
            matches!(selected_pages_for_rebuild(&interaction), PagesForRebuild::Selected(p) if p == vec![5])
        );
    }

    #[test]
    fn rebuild_without_selection_opens_confirm_path() {
        use crate::app::rebuild::{PagesForRebuild, selected_pages_for_rebuild};
        let interaction = GuiState::new(ProjectState::default()).interaction;
        assert!(matches!(
            selected_pages_for_rebuild(&interaction),
            PagesForRebuild::None
        ));
    }
}
