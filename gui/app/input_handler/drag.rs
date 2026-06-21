use crate::task::{BackgroundTask, PagePosMode};

use crate::state::{
    ActiveDrag, ContextMenu, DataState, DragMode, DragSource, HoveredTarget, InteractionState,
    PhotoSelection,
};
use fotobuch::commands::PlaceDst;
use fotobuch::models::{LayoutPage, PageMode};

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
        cursor_mm,
    }) = &interaction.hovered
    {
        let (src_page, src_slot) = (*page, *slot);
        let cursor_mm_at_drag_start = *cursor_mm;
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
            cursor_mm_at_drag_start,
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
        interaction.drag.active = ActiveDrag::Pending {
            source: src,
            press_pos: cursor,
            press_instant: std::time::Instant::now(),
        };
    }
}

/// Promote `Pending` → `Dragging` if the cursor has moved past the threshold.
/// Call each frame while RMB is held, before `handle_drag_complete`.
pub(super) fn promote_pending_drag(interaction: &mut InteractionState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.secondary_down()) {
        return;
    }
    let cursor = match ctx.pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };
    interaction.drag.active.maybe_promote(cursor);
}

/// Handles RMB release for all drag sources.
/// Returns `(action_taken, error_message)` — caller must push any error to `data.toasts`.
pub(super) fn handle_drag_complete(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut Vec<BackgroundTask>,
) -> (bool, Option<String>) {
    if !ctx.input(|i| i.pointer.secondary_released()) {
        return (false, None);
    }

    match std::mem::replace(&mut interaction.drag.active, ActiveDrag::Idle) {
        // RMB released before moving past threshold → it was a tap → context menu.
        ActiveDrag::Pending { .. } => {
            let cursor = ctx.pointer_hover_pos().unwrap_or_default();
            interaction.context_menu = build_context_menu(&interaction.hovered, cursor);
            (true, None)
        }
        ActiveDrag::Dragging(source) => {
            let mut error: Option<String> = None;
            match source {
                DragSource::Slot {
                    src_page,
                    src_slot,
                    src_slots,
                    cursor_mm_at_drag_start,
                    ..
                } => {
                    error = complete_slot_drag(
                        data,
                        interaction,
                        cmds,
                        src_page,
                        src_slot,
                        src_slots,
                        cursor_mm_at_drag_start,
                    );
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
                DragSource::ManualMove {
                    page,
                    slot,
                    pointer_origin,
                    slot_origin_mm,
                    pixel_per_mm,
                } => {
                    let cursor = ctx.pointer_hover_pos().unwrap_or(pointer_origin);
                    let delta_px = cursor - pointer_origin;
                    let dx_mm = delta_px.x as f64 / pixel_per_mm;
                    let dy_mm = delta_px.y as f64 / pixel_per_mm;
                    let new_x = slot_origin_mm.0 + dx_mm;
                    let new_y = slot_origin_mm.1 + dy_mm;
                    if is_slot_visible_on_page(data, page, slot, new_x, new_y) {
                        cmds.push(BackgroundTask::PagePos {
                            page,
                            slot,
                            mode: PagePosMode::Relative { dx_mm, dy_mm },
                            scale: None,
                        });
                    } else if let Some(at_position) = interaction
                        .hovered
                        .as_ref()
                        .and_then(HoveredTarget::new_page_at_position)
                    {
                        cmds.push(BackgroundTask::MoveToNewPage {
                            src_page: page,
                            src_slots: vec![slot],
                            at_position,
                        });
                    } else if let Some(dst_page) = interaction
                        .hovered
                        .as_ref()
                        .and_then(|h| h.central_page())
                        .filter(|&dst| dst != page)
                    {
                        // Cursor is over a different page — cross-page move.
                        cmds.push(BackgroundTask::Move {
                            src_page: page,
                            src_slots: vec![slot],
                            dst_page,
                        });
                    } else {
                        error = Some(
                            "Cannot move slot: it would be completely outside the page.".into(),
                        );
                    }
                }
                DragSource::ManualResize {
                    page,
                    slot,
                    pointer_origin,
                    slot_origin_mm,
                    pixel_per_mm,
                } => {
                    let cursor = ctx.pointer_hover_pos().unwrap_or(pointer_origin);
                    let delta_px = cursor - pointer_origin;
                    let (x_mm, y_mm, new_w, _new_h) =
                        compute_se_resize(slot_origin_mm, delta_px, pixel_per_mm);
                    let orig_w = slot_origin_mm.2;
                    let scale = if orig_w > 0.0 { new_w / orig_w } else { 1.0 };
                    cmds.push(BackgroundTask::PagePos {
                        page,
                        slot,
                        mode: PagePosMode::Absolute { x_mm, y_mm },
                        scale: Some(scale),
                    });
                }
            }
            (true, error)
        }
        ActiveDrag::Idle => (false, None),
    }
}

/// Returns an error message when the drag is rejected, otherwise `None`.
pub(super) fn complete_slot_drag(
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
    src_page: usize,
    src_slot: usize,
    src_slots: Vec<usize>,
    cursor_mm_at_drag_start: (f32, f32),
) -> Option<String> {
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
        return None;
    }

    let src_is_manual = data
        .project
        .layout
        .get(src_page)
        .map(|p| p.mode == PageMode::Manual)
        .unwrap_or(false);

    // On a manual page slots can freely overlap; only move the directly-clicked slot.
    let src_slots = if src_is_manual {
        vec![src_slot]
    } else {
        src_slots
    };

    let hovered_slot = interaction.hovered.as_ref().and_then(|h| h.slot());
    let effective_page = interaction.hovered.as_ref().and_then(|h| h.page_idx());
    let cursor_mm = interaction
        .hovered
        .as_ref()
        .and_then(|h| h.cursor_mm_on_page());

    let dst_is_manual = effective_page
        .and_then(|p| data.project.layout.get(p))
        .map(|p| p.mode == PageMode::Manual)
        .unwrap_or(false);

    match (hovered_slot, interaction.drag.mode) {
        (_, DragMode::Move) if dst_is_manual => {
            // Drop onto a Manual page: keep each moved slot's size and put the
            // dragged photo's upper-left where its drag ghost sits.
            let dst_page = effective_page?;
            if dst_page == src_page {
                return None;
            }
            let (x_mm, y_mm) =
                manual_drop_top_left(data, src_page, src_slot, cursor_mm_at_drag_start, cursor_mm);
            cmds.push(BackgroundTask::MoveToManual {
                src_page,
                src_slots,
                dst_page,
                x_mm,
                y_mm,
            });
        }
        (Some((dst_page, dst_slot)), DragMode::Swap) => {
            if src_slots.len() == 1 {
                // Same-page swap with mismatched aspect ratios is a no-op: auto-mode
                // rebuilds re-sort photos by ratio and would immediately undo the swap.
                let same_page_ratio_mismatch = src_page == dst_page
                    && data.project.layout.get(src_page).is_some_and(|p| {
                        let r = |slot: usize| {
                            p.slots.get(slot).map(|s| s.width_mm / s.height_mm)
                        };
                        matches!((r(src_slots[0]), r(dst_slot)), (Some(a), Some(b)) if (a - b).abs() > 0.01)
                    });
                if !same_page_ratio_mismatch {
                    dispatch_swap(cmds, src_page, src_slots[0], dst_page, dst_slot);
                }
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
    None
}

/// Upper-left position (in dst-page mm) for the dragged slot when dropping onto a
/// Manual page: the cursor's drop point minus the grab offset within the slot, so
/// the slot lands exactly where its drag ghost was shown.
fn manual_drop_top_left(
    data: &DataState,
    src_page: usize,
    src_slot: usize,
    cursor_mm_at_drag_start: (f32, f32),
    drop_cursor_mm: Option<(f32, f32)>,
) -> (f64, f64) {
    let (sx, sy) = data
        .project
        .layout
        .get(src_page)
        .and_then(|p| p.slots.get(src_slot))
        .map(|s| (s.x_mm, s.y_mm))
        .unwrap_or((0.0, 0.0));
    let grab_x = cursor_mm_at_drag_start.0 as f64 - sx;
    let grab_y = cursor_mm_at_drag_start.1 as f64 - sy;
    let (cx, cy) = drop_cursor_mm.unwrap_or(cursor_mm_at_drag_start);
    (cx as f64 - grab_x, cy as f64 - grab_y)
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

fn build_context_menu(hovered: &Option<HoveredTarget>, pos: egui::Pos2) -> Option<ContextMenu> {
    match hovered {
        Some(HoveredTarget::Page {
            page,
            slot: Some(slot),
            ..
        }) => Some(ContextMenu::Slot {
            page: *page,
            slot: *slot,
            screen_pos: pos,
        }),
        Some(HoveredTarget::Page {
            page, slot: None, ..
        }) => Some(ContextMenu::Page {
            page: *page,
            screen_pos: pos,
        }),
        Some(HoveredTarget::NavPage(page)) => Some(ContextMenu::NavPage {
            page: *page,
            screen_pos: pos,
        }),
        Some(HoveredTarget::PoolItem(id)) => Some(ContextMenu::PoolItem {
            id: id.clone(),
            screen_pos: pos,
        }),
        _ => None,
    }
}

/// Returns `true` if placing a slot at `(new_x_mm, new_y_mm)` keeps at least some part of it
/// visible within the page content area.
fn is_slot_visible_on_page(
    data: &DataState,
    page: usize,
    slot: usize,
    new_x_mm: f64,
    new_y_mm: f64,
) -> bool {
    let layout_page = match data.project.layout.get(page) {
        Some(lp) => lp,
        None => return false,
    };
    let slot_data = match layout_page.slots.get(slot) {
        Some(s) => s,
        None => return false,
    };
    let (page_w, page_h) = data.project.page_dimensions_mm(page);
    // The slot rect must overlap [0, page_w] × [0, page_h].
    new_x_mm + slot_data.width_mm > 0.0
        && new_x_mm < page_w
        && new_y_mm + slot_data.height_mm > 0.0
        && new_y_mm < page_h
}

/// SE-corner proportional resize math (mirrors `widgets::central_panel::manual_resize::compute_se`).
fn compute_se_resize(
    origin: (f64, f64, f64, f64),
    delta_px: egui::Vec2,
    pixel_per_mm: f64,
) -> (f64, f64, f64, f64) {
    let (x, y, w, h) = origin;
    let orig_diag = (w * w + h * h).sqrt();
    if orig_diag < f64::EPSILON || pixel_per_mm < f64::EPSILON {
        return origin;
    }
    let dx_mm = delta_px.x as f64 / pixel_per_mm;
    let dy_mm = delta_px.y as f64 / pixel_per_mm;
    let new_se_x = w + dx_mm;
    let new_se_y = h + dy_mm;
    let new_diag = (new_se_x * new_se_x + new_se_y * new_se_y).sqrt();
    let scale = new_diag / orig_diag;
    let min_dim = 1.0_f64;
    let new_w = (w * scale).max(min_dim);
    let new_h = (h * scale).max(min_dim);
    (x, y, new_w, new_h)
}
