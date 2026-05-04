use crate::state::{ActiveDrag, DataState, DragMode, DragSource, InteractionState};

use super::super::geometry::{self, PageDimensions};
use super::{draw_drag_ghosts, helpers};

/// Returns `(hovered_slot, over_page, page_rect)`.
pub(super) fn draw_page(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &InteractionState,
    page_idx: usize,
) -> (Option<usize>, bool, egui::Rect) {
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
        let (hovered_slot, over_page) = hit_test_pointer(ui, page_rect, layout_page, dims);
        super::super::page_nav::draw_nav_selection_overlay(ui, interaction, page_idx, page_rect);
        draw_page_move_highlight(ui, interaction, page_idx, page_rect, over_page);
        draw_pool_drag_overlay(ui, interaction, page_idx, page_rect);
        draw_drag_ghosts::draw_drag_ghosts(ui, data, interaction, page_idx, page_rect, dims);
        (hovered_slot, over_page, page_rect)
    } else {
        (None, false, page_rect)
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
) -> (Option<usize>, bool) {
    match ui.input(|i| i.pointer.latest_pos()) {
        None => (None, false),
        Some(pos) => {
            let slot = geometry::hit_test_slot(pos, page_rect, layout_page, dims);
            (slot, page_rect.contains(pos))
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
