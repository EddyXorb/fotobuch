use crate::state::{ActiveDrag, DataState, DragSource, InteractionState};

use super::super::super::geometry::{self, PageDimensions};
use super::super::manual_resize;

pub(super) fn draw_manual_handles_and_overlay(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
    pixel_per_mm: f64,
) {
    let layout_page = match data.project.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let cursor = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();
    let rmb_pressed = ui.input(|i| i.pointer.secondary_pressed());

    // Free-position move/resize on a manual page works in any drag mode: the
    // Swap/Move toggle only governs slot drags that start on Auto pages.
    if rmb_pressed && matches!(interaction.drag.active, ActiveDrag::Idle) {
        try_start_manual_drag(
            interaction,
            layout_page,
            page_idx,
            page_rect,
            dims,
            cursor,
            pixel_per_mm,
        );
    }

    draw_resize_handles(ui, interaction, layout_page, page_idx, page_rect, dims);
    draw_manual_drag_preview(
        ui,
        interaction,
        layout_page,
        page_idx,
        page_rect,
        dims,
        cursor,
    );
}

fn try_start_manual_drag(
    interaction: &mut InteractionState,
    layout_page: &fotobuch::dto_models::LayoutPage,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
    cursor: egui::Pos2,
    pixel_per_mm: f64,
) {
    for (slot_idx, slot) in layout_page.slots.iter().enumerate().rev() {
        let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
        let se = se_corner_rect(slot_rect);
        let source = if se.contains(cursor) {
            Some(DragSource::ManualResize {
                page: page_idx,
                slot: slot_idx,
                pointer_origin: cursor,
                slot_origin_mm: (slot.x_mm, slot.y_mm, slot.width_mm, slot.height_mm),
                pixel_per_mm,
            })
        } else if slot_rect.contains(cursor) {
            Some(DragSource::ManualMove {
                page: page_idx,
                slot: slot_idx,
                pointer_origin: cursor,
                slot_origin_mm: (slot.x_mm, slot.y_mm),
                pixel_per_mm,
            })
        } else {
            None
        };
        if let Some(src) = source {
            interaction.drag.active = ActiveDrag::Pending {
                source: src,
                press_pos: cursor,
                press_instant: std::time::Instant::now(),
            };
            break;
        }
    }
}

fn draw_resize_handles(
    ui: &mut egui::Ui,
    interaction: &InteractionState,
    layout_page: &fotobuch::dto_models::LayoutPage,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
) {
    let manual_dragging = matches!(
        &interaction.drag.active,
        ActiveDrag::Dragging(DragSource::ManualMove { page, .. } | DragSource::ManualResize { page, .. })
            if *page == page_idx
    );
    if manual_dragging {
        return;
    }
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

fn draw_manual_drag_preview(
    ui: &mut egui::Ui,
    interaction: &InteractionState,
    layout_page: &fotobuch::dto_models::LayoutPage,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
    cursor: egui::Pos2,
) {
    match &interaction.drag.active {
        ActiveDrag::Dragging(DragSource::ManualMove {
            page,
            slot,
            pointer_origin,
            slot_origin_mm,
            pixel_per_mm: ppm,
        }) if *page == page_idx => {
            let delta_px = cursor - *pointer_origin;
            let dx_mm = delta_px.x as f64 / ppm;
            let dy_mm = delta_px.y as f64 / ppm;
            if let Some(slot_data) = layout_page.slots.get(*slot) {
                let preview = fotobuch::dto_models::Slot {
                    x_mm: slot_origin_mm.0 + dx_mm,
                    y_mm: slot_origin_mm.1 + dy_mm,
                    width_mm: slot_data.width_mm,
                    height_mm: slot_data.height_mm,
                };
                let r = geometry::slot_rect_on_screen(page_rect, dims, &preview);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 128, 0)),
                    egui::StrokeKind::Outside,
                );
            }
        }
        ActiveDrag::Dragging(DragSource::ManualResize {
            page,
            slot,
            pointer_origin,
            slot_origin_mm,
            pixel_per_mm: ppm,
        }) if *page == page_idx => {
            let delta_px = cursor - *pointer_origin;
            let (_, _, new_w, new_h) = manual_resize::compute_se(*slot_origin_mm, delta_px, *ppm);
            if let Some(slot_data) = layout_page.slots.get(*slot) {
                let preview = fotobuch::dto_models::Slot {
                    x_mm: slot_origin_mm.0,
                    y_mm: slot_origin_mm.1,
                    width_mm: new_w,
                    height_mm: new_h,
                };
                let r = geometry::slot_rect_on_screen(page_rect, dims, &preview);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 255)),
                    egui::StrokeKind::Outside,
                );
                let _ = slot_data;
            }
            let _ = new_h;
        }
        _ => {}
    }
}

fn se_corner_rect(slot_rect: egui::Rect) -> egui::Rect {
    const SZ: f32 = 8.0;
    egui::Rect::from_center_size(slot_rect.right_bottom(), egui::vec2(SZ, SZ))
}
