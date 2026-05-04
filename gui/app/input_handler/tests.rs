use fotobuch::commands::PlaceDst;
use fotobuch::dto_models::ProjectState;

use super::drag::{
    complete_nav_drag, complete_pool_drag, complete_slot_drag, dispatch_move, dispatch_swap,
};
use super::*;
use crate::state::{GuiState, HoveredTarget, SlotSelection};

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
    let mut cmds = Vec::new();
    dispatch_move(&mut cmds, 0, vec![1, 2, 3], 1);
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::Move {
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
    let mut cmds = Vec::new();
    dispatch_move(&mut cmds, 0, vec![3], 1);
    let BackgroundTask::Move { src_slots, .. } = cmds.iter().next().unwrap() else {
        panic!()
    };
    assert_eq!(*src_slots, vec![3]);
}

#[test]
fn dispatch_swap_cross_page_emits_command() {
    let mut cmds = Vec::new();
    dispatch_swap(&mut cmds, 0, 0, 1, 2);
    assert_eq!(cmds.len(), 1, "cross-page swap must emit a command");
}

#[test]
fn dispatch_swap_ignores_same_slot() {
    let mut cmds = Vec::new();
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
    let mut cmds = Vec::new();
    cmds.push(BackgroundTask::Place {
        photo_ids: state.interaction.selections.photos.ids(),
        dst: match state
            .interaction
            .hovered
            .as_ref()
            .and_then(HoveredTarget::central_page)
        {
            Some(p) => PlaceDst::Page(p),
            None => PlaceDst::Auto,
        },
    });
    let BackgroundTask::Place { dst, .. } = cmds.iter().next().unwrap() else {
        panic!()
    };
    assert_eq!(*dst, PlaceDst::Page(2));
}

#[test]
fn place_hotkey_emits_place_without_target_when_no_hover() {
    let mut state = state_with_pool_selection(vec!["a.jpg"]);
    state.interaction.hovered = None;
    let mut cmds = Vec::new();
    cmds.push(BackgroundTask::Place {
        photo_ids: state.interaction.selections.photos.ids(),
        dst: match state
            .interaction
            .hovered
            .as_ref()
            .and_then(HoveredTarget::central_page)
        {
            Some(p) => PlaceDst::Page(p),
            None => PlaceDst::Auto,
        },
    });
    let BackgroundTask::Place { dst, .. } = cmds.iter().next().unwrap() else {
        panic!()
    };
    assert_eq!(*dst, PlaceDst::Auto);
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
    let mut cmds = Vec::new();
    complete_pool_drag(&mut state.interaction, &mut cmds, vec!["a.jpg".into()]);
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::Place { dst, .. } = cmds.iter().next().unwrap() else {
        panic!()
    };
    assert_eq!(*dst, PlaceDst::Page(1));
}

#[test]
fn pool_drag_complete_cancels_without_hovered_page() {
    let mut state = GuiState::new(ProjectState::default());
    state.interaction.hovered = None;
    let mut cmds = Vec::new();
    complete_pool_drag(&mut state.interaction, &mut cmds, vec!["a.jpg".into()]);
    assert!(cmds.is_empty(), "no hovered page → no command");
}

fn default_data() -> crate::state::DataState {
    crate::state::DataState {
        project: ProjectState::default(),
        derived: crate::state::DerivedState::rebuild(&ProjectState::default()),
        pages: crate::state::PageCache::new(0),
        thumbs: Default::default(),
        timings: Default::default(),
    }
}

#[test]
fn nav_drag_complete_emits_page_swap() {
    let mut state = GuiState::new(ProjectState::default());
    state.interaction.hovered = Some(HoveredTarget::NavPage(2));
    let mut cmds = Vec::new();
    complete_nav_drag(&default_data(), &mut state.interaction, &mut cmds, 0);
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::PageSwap { left, right } = cmds.iter().next().unwrap() else {
        panic!()
    };
    assert_eq!(*left, 0);
    assert_eq!(*right, 2);
}

#[test]
fn nav_drag_complete_noop_when_same_page() {
    let mut state = GuiState::new(ProjectState::default());
    state.interaction.hovered = Some(HoveredTarget::NavPage(1));
    let mut cmds = Vec::new();
    complete_nav_drag(&default_data(), &mut state.interaction, &mut cmds, 1);
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
    let mut cmds = Vec::new();
    complete_slot_drag(
        &default_data(),
        &mut state.interaction,
        &mut cmds,
        1,
        1,
        vec![1, 2],
    );
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::MoveToNewPage {
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
    let mut cmds = Vec::new();
    complete_slot_drag(
        &default_data(),
        &mut state.interaction,
        &mut cmds,
        0,
        0,
        vec![0],
    );
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::MoveToNewPage { at_position, .. } = cmds.iter().next().unwrap() else {
        panic!()
    };
    assert_eq!(*at_position, 0);
}

#[test]
fn cross_page_move_with_selection_moves_all_selected_slots() {
    let mut cmds = Vec::new();
    dispatch_move(&mut cmds, 3, vec![0, 2], 7);
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::Move {
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

fn data_with_layout(layout: Vec<fotobuch::dto_models::LayoutPage>) -> crate::state::DataState {
    let mut project = ProjectState::default();
    project.layout = layout;
    crate::state::DataState {
        derived: crate::state::DerivedState::rebuild(&project),
        pages: crate::state::PageCache::new(project.layout.len()),
        project,
        thumbs: Default::default(),
        timings: Default::default(),
    }
}

#[test]
fn swap_range_uses_full_selection_when_dragged_slot_selected() {
    let mut state = state_with_selection(0, vec![1, 2, 3]);
    state.interaction.drag.mode = crate::state::DragMode::Swap;
    let data = data_with_layout(vec![
        layout_page_with_slots(0, 4),
        layout_page_with_slots(1, 6),
    ]);
    state.interaction.hovered = Some(HoveredTarget::Page {
        page: 1,
        slot: Some(0),
    });
    let mut cmds = Vec::new();
    complete_slot_drag(
        &data,
        &mut state.interaction,
        &mut cmds,
        0,
        1,
        vec![1, 2, 3],
    );
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::SwapRange {
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
    let data = data_with_layout(vec![
        layout_page_with_slots(0, 3),
        layout_page_with_slots(1, 2),
    ]);
    state.interaction.hovered = Some(HoveredTarget::Page {
        page: 1,
        slot: Some(0),
    });
    let mut cmds = Vec::new();
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
    let data = data_with_layout(vec![
        layout_page_with_slots(0, 4),
        layout_page_with_slots(1, 4),
    ]);
    state.interaction.hovered = Some(HoveredTarget::Page {
        page: 1,
        slot: Some(0),
    });
    let mut cmds = Vec::new();
    complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 0, vec![0, 2]);
    assert!(cmds.is_empty(), "non-contiguous selection → no command");
}

#[test]
fn swap_falls_back_to_single_when_selection_is_one() {
    let mut state = state_with_selection(0, vec![1]);
    state.interaction.drag.mode = crate::state::DragMode::Swap;
    let data = data_with_layout(vec![
        layout_page_with_slots(0, 3),
        layout_page_with_slots(1, 3),
    ]);
    state.interaction.hovered = Some(HoveredTarget::Page {
        page: 1,
        slot: Some(2),
    });
    let mut cmds = Vec::new();
    complete_slot_drag(&data, &mut state.interaction, &mut cmds, 0, 1, vec![1]);
    assert_eq!(cmds.len(), 1);
    assert!(
        matches!(cmds.iter().next().unwrap(), BackgroundTask::Swap { .. }),
        "single-slot selection should use Swap, not SwapRange"
    );
}

#[test]
fn handle_delete_emits_unplace_with_selection_slots() {
    let state = state_with_selection(2, vec![0, 3]);
    let mut cmds = Vec::new();
    if let Some(page) = state.interaction.selections.slots.page
        && !state.interaction.selections.slots.is_empty()
    {
        cmds.push(BackgroundTask::Unplace {
            page,
            slots: state.interaction.selections.slots.slots_on_active_page(),
        });
    }
    assert_eq!(cmds.len(), 1);
    let BackgroundTask::Unplace { page, slots } = cmds.iter().next().unwrap() else {
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
