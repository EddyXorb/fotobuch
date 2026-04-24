use std::collections::HashSet;

use egui::Align;

use crate::app::pending::PendingCommand;
use crate::app::widgets::central_panel::draw_drag_ghosts;
use crate::app::widgets::geometry::A4_ASPECT;
use crate::state::{ActiveDrag, DataState, DragSource, HoveredTarget, InteractionState};

pub fn draw(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
) {
    egui::Panel::right("page_nav")
        .resizable(true)
        .min_size(100.0)
        .max_size(200.0)
        .default_size(120.0)
        .show_inside(ui, |ui| show(ui, data, interaction, cmds));
}

fn show(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    _cmds: &mut HashSet<PendingCommand>,
) {
    let panel_width = ui.available_width();
    let num_pages = data.pages.thumb_textures.len();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for i in 0..num_pages {
                let thumb_size = compute_thumb_size(data, i, panel_width);
                let (response, painter) =
                    ui.allocate_painter(thumb_size, egui::Sense::click_and_drag());
                let rect = response.rect;

                draw_thumb(data, i, rect, &painter);
                ui.label(egui::RichText::new(format!("P{i}")).small());
                draw_highlights(interaction, i, rect, &painter, &response);

                if response.hovered() {
                    interaction.hovered = Some(HoveredTarget::NavPage(i));
                }

                if response.clicked() {
                    on_click(interaction, i);
                }

                ui.add_space(4.0);
            }
        });

    draw_drag_ghosts::draw_nav_drag_ghost(ui.ctx(), data, interaction);
}

fn draw_thumb(data: &DataState, page_idx: usize, rect: egui::Rect, painter: &egui::Painter) {
    if let Some(Some(tex)) = data.pages.thumb_textures.get(page_idx) {
        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        painter.rect_filled(rect, 2.0, egui::Color32::from_gray(180));
    }
}

fn draw_highlights(
    interaction: &InteractionState,
    page_idx: usize,
    rect: egui::Rect,
    painter: &egui::Painter,
    response: &egui::Response,
) {
    let is_scroll_target = interaction.viewport.scroll_to_page == Some(page_idx);
    let is_nav_drag_target = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::NavPage { .. })
    ) && response.hovered();
    let is_slot_drag_target = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Slot { .. })
    ) && response.hovered();
    let is_pool_drag_target = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Pool { .. })
    ) && response.hovered();

    if is_pool_drag_target {
        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(64, 128, 255, 48),
        );
    }
    if is_scroll_target {
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 150, 255)),
            egui::StrokeKind::Inside,
        );
    }
    if is_nav_drag_target || is_slot_drag_target {
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 80)),
            egui::StrokeKind::Inside,
        );
    }
}

fn on_click(interaction: &mut InteractionState, page_idx: usize) {
    interaction.viewport.scroll_to_page = Some(page_idx);
    interaction.selections.slots.clear();
}

/// Computes thumb display size, preserving page aspect ratio at panel width - margin.
fn compute_thumb_size(data: &DataState, page_idx: usize, panel_width: f32) -> egui::Vec2 {
    let (pw, ph) = data.project.page_dimensions_mm(page_idx);
    let w = (panel_width - 8.0).max(20.0);
    let h = if pw > 0.0 {
        w * (ph as f32 / pw as f32)
    } else {
        w * A4_ASPECT
    };

    if let Some(Some(tex)) = data.pages.thumb_textures.get(page_idx) {
        let sz = tex.size_vec2();
        if sz.x > 0.0 {
            return egui::vec2(w, w * sz.y / sz.x);
        }
    }

    egui::vec2(w, h)
}

/// Applies the scroll-to-page request if this page rect is known.
///
/// Called from `draw_pages` once per page after laying it out.
pub fn apply_scroll_if_needed(
    ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
) {
    if interaction.viewport.scroll_to_page == Some(page_idx) {
        ui.scroll_to_rect(page_rect, Some(Align::TOP));
        interaction.viewport.scroll_to_page = None;
    }
}
