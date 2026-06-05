use egui::vec2;

use crate::state::{ActiveDrag, DataState, DragMode, DragSource, InteractionState};

use super::super::geometry::{PageDimensions, PageScale};

const STACK_STEP: egui::Vec2 = egui::vec2(10.0, -10.0);

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
            ..
        }) if *src_page == page_idx => (*src_slot, *cursor_at_drag_start),
        _ => return,
    };

    // No ghosts for manual pages in swap mode — swap is disabled there.
    use fotobuch::dto_models::PageMode;
    if interaction.drag.mode == DragMode::Swap
        && data
            .project
            .layout
            .get(page_idx)
            .map(|p| p.mode == PageMode::Manual)
            .unwrap_or(false)
    {
        return;
    }
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

    // Determine the ghost rect for the drag source slot.
    let primary_rect = match layout_page.slots.get(src_slot_idx) {
        Some(slot) => {
            calc_primary_ghost_rect(page_rect, &scale, slot, cursor, cursor_at_drag_start)
        }
        None => return,
    };

    // Build the ordered render list: secondary slots first (bottom), source last (top).
    // Only include secondary slots in Move mode (Swap doesn't multi-select meaningfully).
    let mut render_order: Vec<usize> = Vec::new();
    if interaction.drag.mode != DragMode::Swap {
        let slots_sel = &interaction.selections.slots;
        if slots_sel.page == Some(page_idx) {
            let mut secondary: Vec<usize> = slots_sel
                .slots_on_active_page()
                .into_iter()
                .filter(|&s| s != src_slot_idx)
                .collect();
            render_order.append(&mut secondary);
        }
    }
    render_order.push(src_slot_idx);

    // Render from bottom (index 0) to top (last). Source is last → on top.
    // Stack offset: each layer below the source is shifted by STACK_STEP per step below top.
    let top_idx = render_order.len() - 1;
    for (layer, &slot_idx) in render_order.iter().enumerate() {
        // 0 = source (top), higher = deeper in stack.
        let depth = top_idx - layer;
        let offset = STACK_STEP * depth as f32;
        let rect = primary_rect.translate(offset);
        let alpha: u8 = if depth == 0 { 180 } else { 120 };

        // Try to paint the actual photo thumbnail.
        let photo_id = layout_page
            .photos
            .get(slot_idx)
            .map(|s| s.as_str())
            .unwrap_or("");
        if let Some(tex) = data.thumbs.get(photo_id) {
            painter.image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
            );
        } else {
            paint_ghost_rect(&painter, rect, alpha);
        }

        // Stroke on every layer.
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)),
            egui::StrokeKind::Middle,
        );
    }

    // Mode label on the topmost ghost.
    painter.text(
        primary_rect.right_bottom() + vec2(6.0, -2.0),
        egui::Align2::LEFT_BOTTOM,
        interaction.drag.mode.label(),
        egui::FontId::proportional(14.0),
        egui::Color32::WHITE,
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_step_is_ne_10px_per_layer() {
        // Each deeper layer steps +10 East, -10 North from the layer above.
        assert_eq!(STACK_STEP, egui::vec2(10.0, -10.0));
    }

    #[test]
    fn source_slot_is_topmost_layer() {
        // The source slot index is always the last element in render_order before render.
        // We verify the conceptual ordering: top_idx == render_order.len() - 1,
        // and depth for source = top_idx - top_idx = 0, so offset = ZERO.
        let render_order = [2usize, 3, 0]; // 0 is source
        let top_idx = render_order.len() - 1;
        let source = *render_order.last().unwrap();
        let source_layer = render_order.iter().position(|&s| s == source).unwrap();
        let depth = top_idx - source_layer;
        assert_eq!(depth, 0, "source must be at depth 0 (no offset)");
    }

    #[test]
    fn single_selection_render_order_has_only_source() {
        // Without secondary slots, render_order is just [src_slot_idx].
        let src = 1usize;
        let secondary: Vec<usize> = vec![];
        let mut order: Vec<usize> = secondary;
        order.push(src);
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], src);
    }
}
