use egui::vec2;

use crate::state::{ActiveDrag, DataState, DragMode, DragSource, InteractionState};

use super::super::geometry::{self, A4_ASPECT, PageDimensions, PageScale};

pub(super) fn draw_drag_ghosts(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
) {
    let (src_slot_idx, cursor_at_drag_start) = match &interaction.drag.active {
        ActiveDrag::Dragging(DragSource::Slot {
            src_page,
            src_slot,
            cursor_at_drag_start,
        }) if *src_page == page_idx => (*src_slot, *cursor_at_drag_start),
        _ => return,
    };
    let cursor = match ui.ctx().pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };
    let layout_page = match data.project.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let scale = dims.page_scale(page_rect);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("drag_ghosts"),
    ));

    if let Some(slot) = layout_page.slots.get(src_slot_idx) {
        let rect = calc_primary_ghost_rect(page_rect, &scale, slot, cursor, cursor_at_drag_start);
        paint_ghost_rect(&painter, rect, 120);
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)),
            egui::StrokeKind::Middle,
        );
        painter.text(
            rect.right_bottom() + vec2(6.0, -2.0),
            egui::Align2::LEFT_BOTTOM,
            interaction.drag.mode.label(),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }

    if interaction.drag.mode == DragMode::Swap {
        return;
    }

    let slots_sel = &interaction.selections.slots;
    if slots_sel.page != Some(page_idx) || slots_sel.slots_on_active_page().len() <= 1 {
        return;
    }
    let secondary: Vec<usize> = slots_sel
        .slots_on_active_page()
        .into_iter()
        .filter(|&s| s != src_slot_idx)
        .collect();

    let rects: Vec<egui::Rect> = secondary
        .iter()
        .filter_map(|&idx| {
            let slot = layout_page.slots.get(idx)?;
            Some(geometry::slot_rect_on_screen(page_rect, dims, slot))
        })
        .collect();

    let max_dist = rects
        .iter()
        .map(|r| r.center().distance(cursor))
        .fold(0.0f32, f32::max)
        .max(1.0);

    for rect in rects {
        let t = (rect.center().distance(cursor) / max_dist).clamp(0.0, 1.0);
        paint_ghost_rect(&painter, rect, (180.0 - 100.0 * t) as u8);
    }
}

fn calc_primary_ghost_rect(
    page_rect: egui::Rect,
    scale: &PageScale,
    slot: &fotobuch::dto_models::Slot,
    cursor: egui::Pos2,
    cursor_at_drag_start: egui::Pos2,
) -> egui::Rect {
    let w = slot.width_mm as f32 * scale.scale_x;
    let h = slot.height_mm as f32 * scale.scale_y;
    let slot_top_left = egui::pos2(
        page_rect.min.x + (scale.offset_mm + slot.x_mm as f32) * scale.scale_x,
        page_rect.min.y + (scale.offset_mm + slot.y_mm as f32) * scale.scale_y,
    );
    let grab = cursor_at_drag_start - slot_top_left;
    egui::Rect::from_min_size(cursor - grab, vec2(w, h))
}

fn paint_ghost_rect(painter: &egui::Painter, rect: egui::Rect, alpha: u8) {
    painter.rect_filled(
        rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(100, 149, 237, alpha),
    );
}

/// Draws a floating page-thumbnail ghost while dragging a nav page.
pub(crate) fn draw_nav_drag_ghost(
    ctx: &egui::Context,
    data: &DataState,
    interaction: &InteractionState,
) {
    let src_page = match &interaction.drag.active {
        ActiveDrag::Dragging(DragSource::NavPage { src_page, .. }) => *src_page,
        _ => return,
    };
    let cursor = match ctx.pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("nav_drag_ghost"),
    ));

    // Ghost size: fixed width, A4-ish aspect or taken from texture
    let ghost_w = 80.0_f32;
    let ghost_h = if let Some(Some(tex)) = data.pages.thumb_textures.get(src_page) {
        let sz = tex.size_vec2();
        if sz.x > 0.0 {
            ghost_w * sz.y / sz.x
        } else {
            ghost_w * A4_ASPECT
        }
    } else {
        ghost_w * A4_ASPECT
    };

    let rect = egui::Rect::from_center_size(cursor, vec2(ghost_w, ghost_h));

    if let Some(Some(tex)) = data.pages.thumb_textures.get(src_page) {
        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
        );
    } else {
        paint_ghost_rect(&painter, rect, 120);
    }
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)),
        egui::StrokeKind::Middle,
    );
}
