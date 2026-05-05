use crate::state::{ActiveDrag, DataState, DragMode, DragSource, InteractionState, ManualDrag};
use crate::task::{BackgroundTask, PagePosMode};

use super::super::geometry::{self, PageDimensions};
use super::{draw_drag_ghosts, helpers};

/// Returns `(hovered_slot, over_page, page_rect, cursor_mm)`.
/// `cursor_mm` is the cursor in page-content mm coordinates (offset by bleed+margin).
pub(super) fn draw_page(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    page_idx: usize,
    cmds: &mut Vec<BackgroundTask>,
) -> (Option<usize>, bool, egui::Rect, (f32, f32)) {
    ui.label(format!("Page {page_idx}"));

    let (width_mm, height_mm) = data.project.page_dimensions_mm(page_idx);
    let (bleed_mm, margin_mm) = data.project.page_bleed_margin_mm(page_idx);
    let dims = PageDimensions {
        width_mm,
        height_mm,
        bleed_mm,
        margin_mm,
    };
    let size = helpers::page_display_size(interaction.viewport.zoom, dims);
    let page_rect = render_page_image(ui, data, page_idx, size);

    if let Some(layout_page) = data.project.layout.get(page_idx) {
        draw_slot_overlays(ui, page_rect, data, interaction, page_idx, dims);
        let (hovered_slot, over_page, cursor_mm) =
            hit_test_pointer(ui, page_rect, layout_page, dims);
        super::super::page_nav::draw_nav_selection_overlay(ui, interaction, page_idx, page_rect);
        draw_page_move_highlight(ui, interaction, page_idx, page_rect, over_page);
        draw_pool_drag_overlay(ui, interaction, page_idx, page_rect);
        draw_drag_ghosts::draw_drag_ghosts(ui, data, interaction, page_idx, page_rect, dims);

        // Manual-mode drag handling (move + SE resize via RMB).
        use fotobuch::dto_models::PageMode;
        if layout_page.mode == PageMode::Manual {
            // pixel_per_mm must use the full physical page width (including bleed),
            // because that is what page_rect.width() represents and what slot_rect_on_screen uses.
            let full_w_mm = dims.width_mm + 2.0 * dims.bleed_mm;
            let pixel_per_mm = if full_w_mm > 0.0 {
                page_rect.width() as f64 / full_w_mm
            } else {
                1.0
            };
            handle_manual_drag(
                ui,
                data,
                interaction,
                page_idx,
                page_rect,
                dims,
                pixel_per_mm,
                cmds,
            );
        }

        // Mode toggle badge — outside the page overlay to avoid capturing drop targets.
        let (label, new_mode) = match layout_page.mode {
            PageMode::Auto => ("[A]", PageMode::Manual),
            PageMode::Manual => ("[M]", PageMode::Auto),
        };
        if ui.small_button(label).clicked() {
            cmds.push(BackgroundTask::SetPageMode {
                page: page_idx,
                mode: new_mode,
            });
        }

        (hovered_slot, over_page, page_rect, cursor_mm)
    } else {
        (None, false, page_rect, (0.0, 0.0))
    }
}

fn render_page_image(
    ui: &mut egui::Ui,
    data: &DataState,
    page_idx: usize,
    size: egui::Vec2,
) -> egui::Rect {
    let rect = if let Some(tex) = &data.pages.textures[page_idx] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size))
            .rect
    } else {
        let (r, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(r, 0.0, egui::Color32::from_gray(200));
        r
    };

    if data.pages.dirty.get(page_idx).copied().unwrap_or(false) {
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(200, 200, 200, 150),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "↻",
            egui::FontId::proportional(rect.height() * 0.15),
            egui::Color32::from_gray(80),
        );
    }

    rect
}

fn draw_slot_overlays(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    data: &DataState,
    interaction: &InteractionState,
    page_idx: usize,
    dims: PageDimensions,
) {
    let layout_page = match data.project.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let is_slot_drag = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Slot { .. })
    );
    let is_swap_drag = is_slot_drag && interaction.drag.mode == DragMode::Swap;

    // Suppress swap overlays when this page or the drag source page is Manual.
    use fotobuch::dto_models::PageMode;
    let src_is_manual = if let ActiveDrag::Dragging(DragSource::Slot { src_page, .. }) =
        &interaction.drag.active
    {
        data.project
            .layout
            .get(*src_page)
            .map(|p| p.mode == PageMode::Manual)
            .unwrap_or(false)
    } else {
        false
    };
    let this_is_manual = data
        .project
        .layout
        .get(page_idx)
        .map(|p| p.mode == PageMode::Manual)
        .unwrap_or(false);
    let is_swap_drag = is_swap_drag && !src_is_manual && !this_is_manual;

    let drag_src_ratio: Option<f64> =
        if let ActiveDrag::Dragging(DragSource::Slot {
            src_page, src_slot, ..
        }) = &interaction.drag.active
        {
            data.project
                .layout
                .get(*src_page)
                .and_then(|p| p.slots.get(*src_slot))
                .map(|s| s.width_mm / s.height_mm)
        } else {
            None
        };

    let painter = ui.painter();
    let pointer_pos = ui.input(|i| i.pointer.latest_pos());
    for (slot_idx, slot) in layout_page.slots.iter().enumerate() {
        let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
        let is_hovered = pointer_pos.map(|p| slot_rect.contains(p)).unwrap_or(false);

        if is_swap_drag {
            let target_ratio = slot.width_mm / slot.height_mm;
            let same_ratio =
                drag_src_ratio.is_some_and(|r| geometry::slot_ratio_similar(r, target_ratio));
            let alpha = if is_hovered { 220 } else { 140 };
            let color = if same_ratio {
                egui::Color32::from_rgba_unmultiplied(0, 200, 80, alpha)
            } else {
                egui::Color32::from_rgba_unmultiplied(220, 50, 50, alpha)
            };
            painter.rect_filled(slot_rect, 0.0, color);
        } else if is_hovered && !is_slot_drag {
            painter.rect_filled(
                slot_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 120, 255, 38),
            );
        }

        if interaction.selections.slots.is_selected(page_idx, slot_idx) {
            painter.rect_stroke(
                slot_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 80)),
                egui::StrokeKind::Middle,
            );
        }
    }
}

fn hit_test_pointer(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    layout_page: &fotobuch::dto_models::LayoutPage,
    dims: PageDimensions,
) -> (Option<usize>, bool, (f32, f32)) {
    match ui.input(|i| i.pointer.latest_pos()) {
        None => (None, false, (0.0, 0.0)),
        Some(pos) => {
            let slot = geometry::hit_test_slot(pos, page_rect, layout_page, dims);
            let over = page_rect.contains(pos);
            let cursor_mm = if over {
                let s = dims.page_scale(page_rect);
                let x_mm = (pos.x - page_rect.min.x) / s.scale_x - s.offset_mm;
                let y_mm = (pos.y - page_rect.min.y) / s.scale_y - s.offset_mm;
                (x_mm, y_mm)
            } else {
                (0.0, 0.0)
            };
            (slot, over, cursor_mm)
        }
    }
}

fn draw_page_move_highlight(
    ui: &mut egui::Ui,
    interaction: &InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
    over_page: bool,
) {
    let is_move_drag = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Slot { .. })
    ) && interaction.drag.mode == DragMode::Move;
    if !is_move_drag || !over_page {
        return;
    }
    let is_src_page = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Slot { src_page, .. }) if src_page == page_idx
    );
    if is_src_page {
        return;
    }
    ui.painter().rect_stroke(
        page_rect,
        0.0,
        egui::Stroke::new(3.0, egui::Color32::from_rgba_unmultiplied(0, 200, 80, 180)),
        egui::StrokeKind::Inside,
    );
}

fn draw_pool_drag_overlay(
    ui: &mut egui::Ui,
    interaction: &InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
) {
    if matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Pool { .. })
    ) && interaction.hovered.as_ref().and_then(|h| h.central_page()) == Some(page_idx)
    {
        ui.painter().rect_filled(
            page_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(64, 128, 255, 48),
        );
    }
}

/// Returns the 8×8 px rect at the SE corner of a slot rect.
fn se_corner_rect(slot_rect: egui::Rect) -> egui::Rect {
    const SZ: f32 = 8.0;
    egui::Rect::from_center_size(slot_rect.right_bottom(), egui::vec2(SZ, SZ))
}

#[allow(clippy::too_many_arguments)]
fn handle_manual_drag(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
    pixel_per_mm: f64,
    cmds: &mut Vec<BackgroundTask>,
) {
    let layout_page = match data.project.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let rmb_pressed = ui.input(|i| i.pointer.secondary_pressed());
    let rmb_down = ui.input(|i| i.pointer.secondary_down());
    let rmb_released = ui.input(|i| i.pointer.secondary_released());
    let cursor = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();

    // On RMB press: pick the topmost (last-drawn = highest index) slot under the cursor.
    if rmb_pressed && matches!(interaction.drag.manual, ManualDrag::Idle) {
        for (slot_idx, slot) in layout_page.slots.iter().enumerate().rev() {
            let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
            let se = se_corner_rect(slot_rect);
            if se.contains(cursor) {
                interaction.drag.manual = ManualDrag::Resize {
                    page: page_idx,
                    slot: slot_idx,
                    pointer_origin: cursor,
                    slot_origin_mm: (slot.x_mm, slot.y_mm, slot.width_mm, slot.height_mm),
                };
                break;
            } else if slot_rect.contains(cursor) {
                interaction.drag.manual = ManualDrag::Move {
                    page: page_idx,
                    slot: slot_idx,
                    pointer_origin: cursor,
                    slot_origin_mm: (slot.x_mm, slot.y_mm),
                };
                break;
            }
        }
    }

    // Draw SE-corner handles only when no manual drag is active (avoid visual clutter).
    if matches!(interaction.drag.manual, ManualDrag::Idle) {
        for slot in &layout_page.slots {
            let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
            let se = se_corner_rect(slot_rect);
            ui.painter().rect_filled(
                se,
                0.0,
                egui::Color32::from_rgba_unmultiplied(255, 200, 0, 200),
            );
        }
    }

    // Draw optimistic overlay for active manual drag.
    match &interaction.drag.manual {
        ManualDrag::Move {
            page,
            slot,
            pointer_origin,
            slot_origin_mm,
        } if *page == page_idx => {
            let delta_px = cursor - *pointer_origin;
            let dx_mm = delta_px.x as f64 / pixel_per_mm;
            let dy_mm = delta_px.y as f64 / pixel_per_mm;
            if let Some(slot_data) = layout_page.slots.get(*slot) {
                let preview_slot = fotobuch::dto_models::Slot {
                    x_mm: slot_origin_mm.0 + dx_mm,
                    y_mm: slot_origin_mm.1 + dy_mm,
                    width_mm: slot_data.width_mm,
                    height_mm: slot_data.height_mm,
                };
                let r = geometry::slot_rect_on_screen(page_rect, dims, &preview_slot);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 128, 0)),
                    egui::StrokeKind::Outside,
                );
            }
        }
        ManualDrag::Resize {
            page,
            slot,
            pointer_origin,
            slot_origin_mm,
        } if *page == page_idx => {
            let delta_px = cursor - *pointer_origin;
            let (_, _, new_w, new_h) =
                super::manual_resize::compute_se(*slot_origin_mm, delta_px, pixel_per_mm);
            if let Some(slot_data) = layout_page.slots.get(*slot) {
                let preview_slot = fotobuch::dto_models::Slot {
                    x_mm: slot_origin_mm.0,
                    y_mm: slot_origin_mm.1,
                    width_mm: new_w,
                    height_mm: new_h,
                };
                let r = geometry::slot_rect_on_screen(page_rect, dims, &preview_slot);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 255)),
                    egui::StrokeKind::Outside,
                );
                let _ = slot_data; // used above
            }
        }
        _ => {}
    }

    // On RMB release: commit the command.
    if rmb_released {
        match std::mem::take(&mut interaction.drag.manual) {
            ManualDrag::Move {
                page,
                slot,
                pointer_origin,
                ..
            } if page == page_idx => {
                let delta_px = cursor - pointer_origin;
                let dx_mm = delta_px.x as f64 / pixel_per_mm;
                let dy_mm = delta_px.y as f64 / pixel_per_mm;
                cmds.push(BackgroundTask::PagePos {
                    page,
                    slot,
                    mode: PagePosMode::Relative { dx_mm, dy_mm },
                    scale: None,
                });
            }
            ManualDrag::Resize {
                page,
                slot,
                pointer_origin,
                slot_origin_mm,
            } if page == page_idx => {
                let delta_px = cursor - pointer_origin;
                let (x_mm, y_mm, new_w, new_h) =
                    super::manual_resize::compute_se(slot_origin_mm, delta_px, pixel_per_mm);
                let orig_w = slot_origin_mm.2;
                let scale = if orig_w > 0.0 { new_w / orig_w } else { 1.0 };
                cmds.push(BackgroundTask::PagePos {
                    page,
                    slot,
                    mode: PagePosMode::Absolute { x_mm, y_mm },
                    scale: Some(scale),
                });
                let _ = new_h;
            }
            other => {
                // Different page or Idle — put it back.
                interaction.drag.manual = other;
            }
        }
    } else if !rmb_down {
        // Cancel if RMB is no longer pressed and no release event (edge case).
        if !matches!(interaction.drag.manual, ManualDrag::Idle) {
            let page = match &interaction.drag.manual {
                ManualDrag::Move { page, .. } | ManualDrag::Resize { page, .. } => *page,
                ManualDrag::Idle => usize::MAX,
            };
            if page == page_idx {
                interaction.drag.manual = ManualDrag::Idle;
            }
        }
    }
}
