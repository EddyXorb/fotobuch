use crate::state::{DragMode, DragSource, DragState, GuiState};

use super::super::geometry::{self, PageDimensions};
use super::{draw_drag_ghosts, helpers};

/// Returns `(hovered_slot, over_page, page_rect)`.
pub(super) fn draw_page(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
) -> (Option<usize>, bool, egui::Rect) {
    ui.label(format!("Page {page_idx}"));

    let (width_mm, height_mm) = state.project_state.page_dimensions_mm(page_idx);
    let (bleed_mm, margin_mm) = state.project_state.page_bleed_margin_mm(page_idx);
    let dims = PageDimensions {
        width_mm,
        height_mm,
        bleed_mm,
        margin_mm,
    };
    let size = helpers::page_display_size(state.zoom, dims);
    let page_rect = render_page_image(ui, state, page_idx, size);

    if let Some(layout_page) = state.project_state.layout.get(page_idx) {
        draw_slot_overlays(ui, page_rect, state, page_idx, dims);
        let (hovered_slot, over_page) = hit_test_pointer(ui, page_rect, layout_page, dims);
        draw_page_move_highlight(ui, state, page_idx, page_rect, over_page);
        draw_pool_drag_overlay(ui, state, page_idx, page_rect);
        draw_drag_ghosts::draw_drag_ghosts(ui, state, page_idx, page_rect, dims);
        (hovered_slot, over_page, page_rect)
    } else {
        (None, false, page_rect)
    }
}

fn render_page_image(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    size: egui::Vec2,
) -> egui::Rect {
    let rect = if let Some(tex) = &state.page_textures[page_idx] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size))
            .rect
    } else {
        let (r, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(r, 0.0, egui::Color32::from_gray(200));
        r
    };

    if state.page_dirty.get(page_idx).copied().unwrap_or(false) {
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
    state: &GuiState,
    page_idx: usize,
    dims: PageDimensions,
) {
    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let is_slot_drag = matches!(state.drag, DragState::Dragging(DragSource::Slot { .. }));
    let is_swap_drag = is_slot_drag && state.drag_mode == DragMode::Swap;

    let drag_src_ratio: Option<f64> =
        if let DragState::Dragging(DragSource::Slot {
            src_page, src_slot, ..
        }) = &state.drag
        {
            if *src_page == page_idx {
                layout_page
                    .slots
                    .get(*src_slot)
                    .map(|s| s.width_mm / s.height_mm)
            } else {
                None
            }
        } else {
            None
        };

    let painter = ui.painter();
    for (slot_idx, slot) in layout_page.slots.iter().enumerate() {
        let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
        let is_hovered =
            state.hovered.as_ref().and_then(|h| h.slot()) == Some((page_idx, slot_idx));

        if is_swap_drag {
            let target_ratio = slot.width_mm / slot.height_mm;
            let same_ratio =
                drag_src_ratio.is_some_and(|r| geometry::slot_ratio_similar(r, target_ratio));
            let color = if same_ratio {
                egui::Color32::from_rgba_unmultiplied(0, 200, 80, 140)
            } else {
                egui::Color32::from_rgba_unmultiplied(220, 50, 50, 140)
            };
            painter.rect_filled(slot_rect, 0.0, color);
        } else if is_hovered && !is_slot_drag {
            painter.rect_filled(
                slot_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 120, 255, 38),
            );
        }

        if state.selections.slots.is_selected(page_idx, slot_idx) {
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
) -> (Option<usize>, bool) {
    match ui.ctx().pointer_hover_pos() {
        None => (None, false),
        Some(pos) => {
            let slot = geometry::hit_test_slot(pos, page_rect, layout_page, dims);
            (slot, page_rect.contains(pos))
        }
    }
}

fn draw_page_move_highlight(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
    over_page: bool,
) {
    let is_move_drag = matches!(state.drag, DragState::Dragging(DragSource::Slot { .. }))
        && state.drag_mode == DragMode::Move;
    if !is_move_drag || !over_page {
        return;
    }
    let is_src_page = matches!(
        state.drag,
        DragState::Dragging(DragSource::Slot { src_page, .. }) if src_page == page_idx
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
    state: &GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
) {
    if matches!(state.drag, DragState::Dragging(DragSource::Pool { .. }))
        && state.hovered.as_ref().and_then(|h| h.central_page()) == Some(page_idx)
    {
        ui.painter().rect_filled(
            page_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(64, 128, 255, 48),
        );
    }
}
