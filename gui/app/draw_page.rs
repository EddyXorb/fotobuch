use egui::vec2;

use crate::state::{DragState, GuiState, slot_ratio_similar};

use super::geometry;

/// Draw a single page at index `i`.
///
/// Returns the slot index the pointer is hovering over (if any).
/// Overlays are drawn with the previous frame's `hovered_slot` and `selection` — one-frame lag,
/// standard egui practice.
pub fn draw_page(ui: &mut egui::Ui, state: &GuiState, page_idx: usize) -> Option<usize> {
    ui.label(format!("Page {page_idx}"));

    let (page_width_mm, page_height_mm) = state.project_state.page_dimensions_mm(page_idx);
    let size = page_display_size(state.zoom, page_width_mm, page_height_mm);
    let page_rect = render_page_image(ui, state, page_idx, size);

    if let Some(layout_page) = state.project_state.layout.get(page_idx) {
        draw_slot_overlays(
            ui,
            page_rect,
            state,
            page_idx,
            page_width_mm,
            page_height_mm,
        );
        let hovered = hit_test_pointer(ui, page_rect, layout_page, page_width_mm, page_height_mm);
        draw_drag_ghost(
            ui,
            state,
            page_idx,
            page_rect,
            page_width_mm,
            page_height_mm,
        );
        hovered
    } else {
        None
    }
}

/// Computes the on-screen size of a page in egui points.
fn page_display_size(zoom: f32, page_width_mm: f64, page_height_mm: f64) -> egui::Vec2 {
    let mm_to_pt = 72.0_f32 / 25.4_f32;
    vec2(
        page_width_mm as f32 * mm_to_pt * zoom,
        page_height_mm as f32 * mm_to_pt * zoom,
    )
}

/// Renders the page texture or a grey placeholder. Returns the allocated page rect.
///
/// If the page is dirty, draws an overlay + loading indicator over the existing texture.
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

    // Loading overlay when page is dirty (command in progress).
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

/// Paints hover and selection overlays for each slot.
///
/// During a drag: the hovered target slot is coloured green (same ratio) or red (different).
fn draw_slot_overlays(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    state: &GuiState,
    page_idx: usize,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    // Precompute drag source ratio for target-slot colour during drag.
    let drag_src_ratio: Option<f64> = if let DragState::Dragging {
        src_page, src_slot, ..
    } = &state.drag
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

    let is_dragging = !matches!(state.drag, DragState::Idle);

    let painter = ui.painter();
    for (slot_idx, slot) in layout_page.slots.iter().enumerate() {
        let slot_rect =
            geometry::slot_rect_on_screen(page_rect, page_width_mm, page_height_mm, slot);

        let is_hovered = state.hovered_slot == Some((page_idx, slot_idx));

        if is_dragging && is_hovered {
            // Ratio feedback: green = same ratio, red = different.
            let target_ratio = slot.width_mm / slot.height_mm;
            let same_ratio = drag_src_ratio.is_some_and(|r| slot_ratio_similar(r, target_ratio));
            let color = if same_ratio {
                egui::Color32::from_rgba_unmultiplied(0, 200, 80, 60)
            } else {
                egui::Color32::from_rgba_unmultiplied(220, 50, 50, 60)
            };
            painter.rect_filled(slot_rect, 0.0, color);
        } else if is_hovered {
            painter.rect_filled(
                slot_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 120, 255, 38),
            );
        }

        if state.selection.is_selected(page_idx, slot_idx) {
            painter.rect_stroke(
                slot_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 80)),
                egui::StrokeKind::Middle,
            );
        }
    }
}

/// Draws the drag ghost (semi-transparent slot shape at cursor) on the source page.
fn draw_drag_ghost(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let (src_slot_idx, _is_move) = match &state.drag {
        DragState::Dragging {
            src_page,
            src_slot,
            is_move,
        } if *src_page == page_idx => (*src_slot, *is_move),
        _ => return,
    };

    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let slot = match layout_page.slots.get(src_slot_idx) {
        Some(s) => s,
        None => return,
    };

    let cursor = match ui.ctx().pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };

    // Size of ghost = same as source slot in screen units.
    let scale_x = page_rect.width() / page_width_mm as f32;
    let scale_y = page_rect.height() / page_height_mm as f32;
    let w = slot.width_mm as f32 * scale_x;
    let h = slot.height_mm as f32 * scale_y;

    let ghost_rect = egui::Rect::from_center_size(cursor, vec2(w, h));

    // Use a top-level painter so the ghost appears above all page content.
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("drag_ghost"),
    ));

    painter.rect_filled(
        ghost_rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(100, 149, 237, 120),
    );
    painter.rect_stroke(
        ghost_rect,
        4.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)),
        egui::StrokeKind::Middle,
    );
}

/// Hit-tests the current pointer position against the page's slots. Returns the slot index or
/// `None` when the pointer is outside the page or between slots.
fn hit_test_pointer(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    layout_page: &fotobuch::dto_models::LayoutPage,
    page_width_mm: f64,
    page_height_mm: f64,
) -> Option<usize> {
    ui.ctx().pointer_hover_pos().and_then(|pos| {
        geometry::hit_test_slot(pos, page_rect, layout_page, page_width_mm, page_height_mm)
    })
}
