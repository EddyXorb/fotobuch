use crate::state::{ActiveDrag, DataState, DragMode, DragSource, InteractionState};

use super::super::super::geometry::{self, PageDimensions};

pub(super) fn draw_slot_overlays(
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

    use fotobuch::models::PageMode;
    let src_is_manual =
        if let ActiveDrag::Dragging(DragSource::Slot { src_page, .. }) = &interaction.drag.active {
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

        draw_slot_flash(ui, interaction, page_idx, slot_idx, slot_rect);
    }
}

/// Brief pulsing highlight on the slot a clicked pool photo was scrolled to.
fn draw_slot_flash(
    ui: &egui::Ui,
    interaction: &InteractionState,
    page_idx: usize,
    slot_idx: usize,
    slot_rect: egui::Rect,
) {
    use crate::state::{FLASH_DURATION, flash_intensity};

    let Some(flash) = &interaction.viewport.flash else {
        return;
    };
    if flash.page != page_idx || flash.slot != slot_idx {
        return;
    }
    let elapsed = ui.ctx().input(|i| i.time) - flash.start;
    let Some(intensity) = flash_intensity(elapsed, FLASH_DURATION) else {
        return;
    };

    const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x88, 0x40);
    let fill_alpha = (intensity * 210.0) as u8;
    let stroke_alpha = (intensity * 255.0) as u8;
    let painter = ui.painter();
    painter.rect_filled(
        slot_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), fill_alpha),
    );
    painter.rect_stroke(
        slot_rect,
        0.0,
        egui::Stroke::new(
            3.0,
            egui::Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), stroke_alpha),
        ),
        egui::StrokeKind::Middle,
    );
    ui.ctx().request_repaint();
}

pub(super) fn draw_page_move_highlight(
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

pub(super) fn draw_pool_drag_overlay(
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
