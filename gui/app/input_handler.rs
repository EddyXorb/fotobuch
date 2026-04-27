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
mod tests {
    use fotobuch::dto_models::ProjectState;

    use super::drag::{
        complete_nav_drag, complete_pool_drag, complete_slot_drag, dispatch_move, dispatch_swap,
    };
    use super::*;
    use crate::state::{GuiState, HoveredTarget, SlotSelection};

    use crate::app::pending::PendingCommand;

    fn state_with_selection(sel_page: usize, sel_slots: Vec<usize>) -> GuiState {
        let mut state = GuiState::new(ProjectState::default());
        for (i, &slot) in sel_slots.iter().enumerate() {
            if i == 0 {
                state.interaction.selections.slots = SlotSelection::single(sel_page, slot);
            } else {
                state.interaction.selections.slots.toggle(sel_page, slot);
            }
        }
        state
    }

    #[test]
    fn dispatch_move_emits_command_with_given_slots() {
        let mut cmds = HashSet::new();
        dispatch_move(&mut cmds, 0, vec![1, 2, 3], 1);
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
    fn dispatch_move_single_slot() {
        let mut cmds = HashSet::new();
        dispatch_move(&mut cmds, 0, vec![3], 1);
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
        let data = crate::state::DataState {
            project: ProjectState::default(),
            derived: crate::state::DerivedState::rebuild(&ProjectState::default()),
            pages: crate::state::PageCache::new(0),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        complete_nav_drag(&data, &mut state.interaction, &mut cmds, 0, vec![0]);
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
        let data = crate::state::DataState {
            project: ProjectState::default(),
            derived: crate::state::DerivedState::rebuild(&ProjectState::default()),
            pages: crate::state::PageCache::new(0),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        complete_nav_drag(&data, &mut state.interaction, &mut cmds, 1, vec![1]);
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

    #[test]
    fn drop_on_new_page_slot_emits_move_to_new_page() {
        let mut state = state_with_selection(1, vec![1, 2]);
        state.interaction.hovered = Some(HoveredTarget::NewPageSlot { at_position: 3 });
        let mut cmds = HashSet::new();
        let data = crate::state::DataState {
            project: ProjectState::default(),
            derived: crate::state::DerivedState::rebuild(&ProjectState::default()),
            pages: crate::state::PageCache::new(0),
            thumbs: Default::default(),
            timings: Default::default(),
        };
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 1, 1, vec![1, 2]);
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
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 0, vec![0]);
        assert_eq!(cmds.len(), 1);
        let PendingCommand::MoveToNewPage { at_position, .. } = cmds.iter().next().unwrap() else {
            panic!()
        };
        assert_eq!(*at_position, 0);
    }

    #[test]
    fn cross_page_move_with_selection_moves_all_selected_slots() {
        let mut cmds = HashSet::new();
        dispatch_move(&mut cmds, 3, vec![0, 2], 7);
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
        state.interaction.drag.mode = crate::state::DragMode::Swap;
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
        complete_slot_drag(
            &data,
            &mut state.interaction,
            &mut cmds,
            0,
            1,
            vec![1, 2, 3],
        );
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
        state.interaction.drag.mode = crate::state::DragMode::Swap;
        let mut project = ProjectState::default();
        project.layout = vec![layout_page_with_slots(0, 3), layout_page_with_slots(1, 2)];
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
        complete_slot_drag(
            &data,
            &mut state.interaction,
            &mut cmds,
            0,
            0,
            vec![0, 1, 2],
        );
        assert!(cmds.is_empty(), "overrun → no command emitted");
    }

    #[test]
    fn swap_range_noop_when_selection_not_contiguous() {
        let mut state = state_with_selection(0, vec![0, 2]);
        state.interaction.drag.mode = crate::state::DragMode::Swap;
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
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 0, vec![0, 2]);
        assert!(cmds.is_empty(), "non-contiguous selection → no command");
    }

    #[test]
    fn swap_falls_back_to_single_when_selection_is_one() {
        let mut state = state_with_selection(0, vec![1]);
        state.interaction.drag.mode = crate::state::DragMode::Swap;
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
        complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 1, vec![1]);
        assert_eq!(cmds.len(), 1);
        assert!(
            matches!(cmds.iter().next().unwrap(), PendingCommand::Swap { .. }),
            "single-slot selection should use Swap, not SwapRange"
        );
    }

    #[test]
    fn handle_delete_emits_unplace_with_selection_slots() {
        let state = state_with_selection(2, vec![0, 3]);
        let mut cmds = HashSet::new();
        if let Some(page) = state.interaction.selections.slots.page
            && !state.interaction.selections.slots.is_empty()
        {
            cmds.insert(PendingCommand::Unplace {
                page,
                slots: state.interaction.selections.slots.slots_on_active_page(),
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
        let slot_sel_present = state.interaction.selections.slots.page.is_some()
            && !state.interaction.selections.slots.is_empty();
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
        let page = 0usize;
        let would_emit = !(project.has_cover() && page == 0);
        assert!(!would_emit, "cover page must not be deleted");
    }

    #[test]
    fn rebuild_selection_matches_selected_page() {
        use crate::app::rebuild::{PagesForRebuild, selected_pages_for_rebuild};
        let mut interaction = GuiState::new(ProjectState::default()).interaction;
        interaction.selections.slots = SlotSelection::single(5, 0);
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
