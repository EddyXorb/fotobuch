use egui::vec2;

use crate::state::{DragMode, DragSource, DragState, GuiState, Selection};

use super::super::geometry;

pub(super) fn draw_drag_ghosts(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let (src_slot_idx, cursor_at_drag_start) = match &state.drag {
        DragState::Dragging(DragSource::Slot {
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
    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let scale_x = page_rect.width() / page_width_mm as f32;
    let scale_y = page_rect.height() / page_height_mm as f32;
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("drag_ghosts"),
    ));

    if let Some(slot) = layout_page.slots.get(src_slot_idx) {
        let rect = calc_primary_ghost_rect(
            page_rect,
            scale_x,
            scale_y,
            slot,
            cursor,
            cursor_at_drag_start,
        );
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
            state.drag_mode.label(),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }

    if state.drag_mode == DragMode::Swap {
        return;
    }

    let secondary: Vec<usize> = match &state.selection {
        Selection::OnPage { page, slots, .. } if *page == page_idx && slots.len() > 1 => slots
            .iter()
            .filter(|&&s| s != src_slot_idx)
            .copied()
            .collect(),
        _ => return,
    };

    let rects: Vec<egui::Rect> = secondary
        .iter()
        .filter_map(|&idx| {
            let slot = layout_page.slots.get(idx)?;
            Some(geometry::slot_rect_on_screen(
                page_rect,
                page_width_mm,
                page_height_mm,
                slot,
            ))
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
    scale_x: f32,
    scale_y: f32,
    slot: &fotobuch::dto_models::Slot,
    cursor: egui::Pos2,
    cursor_at_drag_start: egui::Pos2,
) -> egui::Rect {
    let w = slot.width_mm as f32 * scale_x;
    let h = slot.height_mm as f32 * scale_y;
    let slot_top_left = egui::pos2(
        page_rect.min.x + slot.x_mm as f32 * scale_x,
        page_rect.min.y + slot.y_mm as f32 * scale_y,
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
