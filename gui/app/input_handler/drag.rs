use crate::task::BackgroundTask;

use crate::state::{
    ActiveDrag, DataState, DragMode, DragSource, HoveredTarget, InteractionState, PhotoSelection,
};
use fotobuch::commands::PlaceDst;
use fotobuch::dto_models::LayoutPage;

pub(super) fn handle_drag_start(interaction: &mut InteractionState, ctx: &egui::Context) {
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
        let (src_page, src_slot) = (*page, *slot);
        let mut src_slots = if interaction.selections.slots.is_selected(src_page, src_slot)
            && interaction.selections.slots.page == Some(src_page)
        {
            interaction.selections.slots.slots_on_active_page()
        } else {
            vec![src_slot]
        };
        src_slots.sort_unstable();
        Some(DragSource::Slot {
            src_page,
            src_slot,
            src_slots,
            cursor_at_drag_start: cursor,
        })
    } else if let Some(HoveredTarget::NavPage(nav_page)) = &interaction.hovered {
        let nav_page = *nav_page;
        let src_pages = if interaction.selections.nav_pages.is_selected(&nav_page) {
            interaction.selections.nav_pages.items()
        } else {
            vec![nav_page]
        };
        Some(DragSource::NavPage {
            src_page: nav_page,
            src_pages,
        })
    } else if let Some(HoveredTarget::PoolItem(pool_id)) = &interaction.hovered {
        let pool_id = pool_id.clone();
        let ids = if interaction.selections.photos.is_selected(&pool_id) {
            interaction.selections.photos.ids()
        } else {
            interaction.selections.photos = PhotoSelection::single(pool_id.clone());
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
pub(super) fn handle_drag_complete(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut Vec<BackgroundTask>,
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
            src_page,
            src_slot,
            src_slots,
            ..
        } => {
            complete_slot_drag(data, interaction, cmds, src_page, src_slot, src_slots);
        }
        DragSource::NavPage {
            src_page,
            src_pages: _,
            ..
        } => {
            complete_nav_drag(data, interaction, cmds, src_page);
        }
        DragSource::Pool { photo_ids } => {
            complete_pool_drag(interaction, cmds, photo_ids);
        }
    }
    true
}

pub(super) fn complete_slot_drag(
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
    src_page: usize,
    _src_slot: usize,
    src_slots: Vec<usize>,
) {
    if let Some(at_position) = interaction
        .hovered
        .as_ref()
        .and_then(HoveredTarget::new_page_at_position)
    {
        cmds.push(BackgroundTask::MoveToNewPage {
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
            if src_slots.len() == 1 {
                dispatch_swap(cmds, src_page, src_slots[0], dst_page, dst_slot);
            } else if is_contiguous(&src_slots)
                && let Some(layout_dst) = data.project.layout.get(dst_page)
                && let Some(dst_slots) = compute_dst_range(dst_slot, src_slots.len(), layout_dst)
            {
                cmds.push(BackgroundTask::SwapRange {
                    src_page,
                    src_slots,
                    dst_page,
                    dst_slots,
                });
            }
        }
        (Some((dst_page, _)), DragMode::Move) => {
            dispatch_move(cmds, src_page, src_slots, dst_page);
        }
        (None, DragMode::Move) => {
            if let Some(dst_page) = effective_page {
                dispatch_move(cmds, src_page, src_slots, dst_page);
            }
        }
        (None, DragMode::Swap) => {}
    }
}

pub(super) fn complete_nav_drag(
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
    src_page: usize,
) {
    if let Some(at_position) = interaction
        .hovered
        .as_ref()
        .and_then(HoveredTarget::new_page_at_position)
    {
        if interaction.drag.mode == DragMode::Move {
            cmds.push(BackgroundTask::MovePage {
                src_page,
                at_position,
            });
        }
        return;
    }

    if interaction.drag.mode == DragMode::Move {
        if let Some(dst_page) = interaction.hovered.as_ref().and_then(|h| h.page_idx())
            && src_page != dst_page
        {
            let slot_count = data
                .project
                .layout
                .get(src_page)
                .map(|lp| lp.slots.len())
                .unwrap_or(0);
            if slot_count > 0 {
                cmds.push(BackgroundTask::Move {
                    src_page,
                    src_slots: (0..slot_count).collect(),
                    dst_page,
                });
            }
        }
        return;
    }

    // Swap mode always exchanges exactly two pages; src_pages is intentionally ignored here.
    let dst_page = match interaction.hovered.as_ref().and_then(|h| h.as_nav_page()) {
        Some(p) if p != src_page => p,
        _ => return,
    };
    cmds.push(BackgroundTask::PageSwap {
        left: src_page,
        right: dst_page,
    });
}

pub(super) fn complete_pool_drag(
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
    photo_ids: Vec<String>,
) {
    if let Some(pos) = interaction
        .hovered
        .as_ref()
        .and_then(HoveredTarget::new_page_at_position)
    {
        cmds.push(BackgroundTask::Place {
            photo_ids,
            dst: PlaceDst::NewPageAt(pos),
        });
    } else if let Some(dst_page) = interaction.hovered.as_ref().and_then(|h| h.page_idx()) {
        cmds.push(BackgroundTask::Place {
            photo_ids,
            dst: PlaceDst::Page(dst_page),
        });
    }
}

pub(super) fn dispatch_move(
    cmds: &mut Vec<BackgroundTask>,
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
) {
    cmds.push(BackgroundTask::Move {
        src_page,
        src_slots,
        dst_page,
    });
}

pub(super) fn dispatch_swap(
    cmds: &mut Vec<BackgroundTask>,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
) {
    if src_page == dst_page && src_slot == dst_slot {
        return;
    }
    cmds.push(BackgroundTask::Swap {
        src_page,
        src_slot,
        dst_page,
        dst_slot,
    });
}

fn is_contiguous(sorted_slots: &[usize]) -> bool {
    debug_assert!(sorted_slots.windows(2).all(|w| w[0] <= w[1]));
    sorted_slots.windows(2).all(|w| w[1] == w[0] + 1)
}

fn compute_dst_range(dst_slot: usize, count: usize, layout_dst: &LayoutPage) -> Option<Vec<usize>> {
    let end_excl = dst_slot + count;
    if end_excl > layout_dst.slots.len() {
        return None;
    }
    Some((dst_slot..end_excl).collect())
}
