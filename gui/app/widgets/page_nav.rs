use crate::task::BackgroundTask;

use crate::app::widgets::geometry::A4_ASPECT;
use crate::state::{ActiveDrag, DataState, DragMode, DragSource, HoveredTarget, InteractionState};

pub fn draw(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    egui::Panel::right("page_nav")
        .resizable(true)
        .min_size(100.0)
        .max_size(200.0)
        .default_size(120.0)
        .show(ui, |ui| show(ui, data, interaction, cmds));
}

fn show(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    _cmds: &mut Vec<BackgroundTask>,
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
                draw_highlights(interaction, i, rect, &painter);

                if response.hovered() {
                    interaction.hovered = Some(HoveredTarget::NavPage(i));
                }

                if response.clicked() {
                    let modifiers = ui.ctx().input(|inp| inp.modifiers);
                    on_click(interaction, i, num_pages, modifiers);
                }

                ui.add_space(4.0);
            }
        });

    draw_nav_drag_ghost(ui.ctx(), data, interaction);

    let panel_rect = ui.max_rect();
    if ui.rect_contains_pointer(panel_rect) {
        interaction.help.hovered_widget = Some(("nav-panel", panel_rect));
    }
    if interaction.help.highlighted == Some("nav-panel") {
        let time = ui.ctx().input(|i| i.time);
        crate::app::help::draw_glow(ui.painter(), panel_rect, time);
    }
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
) {
    let is_scroll_target = interaction.viewport.scroll_to_page == Some(page_idx);
    let pointer_over = painter
        .ctx()
        .input(|i| i.pointer.latest_pos())
        .map(|p| rect.contains(p))
        .unwrap_or(false);
    let is_nav_drag_target = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::NavPage { .. })
    ) && pointer_over;
    let is_slot_drag_target = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Slot { .. })
    ) && pointer_over
        && interaction.drag.mode == DragMode::Move;
    let is_pool_drag_target = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Pool { .. })
    ) && pointer_over;

    let is_nav_selected = interaction.selections.nav_pages.is_selected(&page_idx);
    if is_nav_selected {
        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 200, 80, 60),
        );
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(0, 180, 60, 200)),
            egui::StrokeKind::Inside,
        );
    }
    if is_pool_drag_target || is_slot_drag_target {
        let alpha = if is_slot_drag_target { 100 } else { 48 };
        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(64, 128, 255, alpha),
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
    if is_nav_drag_target {
        painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 80)),
            egui::StrokeKind::Inside,
        );
    }
}

fn on_click(
    interaction: &mut InteractionState,
    page_idx: usize,
    num_pages: usize,
    modifiers: egui::Modifiers,
) {
    interaction.viewport.scroll_to_page = Some(page_idx);
    interaction.selections.slots.clear();
    if modifiers.shift {
        let order: Vec<usize> = (0..num_pages).collect();
        interaction
            .selections
            .nav_pages
            .range_to_ordered(page_idx, &order);
    } else if modifiers.ctrl || modifiers.command {
        interaction.selections.nav_pages.toggle(page_idx);
    } else {
        interaction.selections.nav_pages = crate::state::MultiSelection::single(page_idx);
    }
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

fn draw_nav_drag_ghost(ctx: &egui::Context, data: &DataState, interaction: &InteractionState) {
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
    let rect = egui::Rect::from_center_size(cursor, egui::vec2(ghost_w, ghost_h));
    if let Some(Some(tex)) = data.pages.thumb_textures.get(src_page) {
        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
        );
    } else {
        painter.rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgba_unmultiplied(100, 149, 237, 120),
        );
    }
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)),
        egui::StrokeKind::Middle,
    );
}

/// Draws the nav-selection overlay on a central-panel page rect.
/// Called from `draw_page` so the logic stays co-located with nav-panel state.
pub(crate) fn draw_nav_selection_overlay(
    ui: &mut egui::Ui,
    interaction: &InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
) {
    if !interaction.selections.nav_pages.is_selected(&page_idx) {
        return;
    }
    ui.painter().rect_filled(
        page_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(0, 200, 80, 40),
    );
    ui.painter().rect_stroke(
        page_rect,
        0.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(0, 180, 60, 200)),
        egui::StrokeKind::Inside,
    );
}

/// Applies the scroll-to-page request if this page rect is known.
///
/// Called from `draw_pages` once per page after laying it out.
pub fn apply_scroll_if_needed(
    _ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
) {
    if interaction.viewport.scroll_to_page == Some(page_idx) {
        // Convert page screen position to a scroll offset and ease to it.
        let target_y = interaction.viewport.scroll.scroll_y
            + (page_rect.min.y - interaction.viewport.scroll.viewport_top);
        interaction.viewport.scroll.ease_target = Some(target_y.max(0.0));
        interaction.viewport.scroll_to_page = None;
    }
}
