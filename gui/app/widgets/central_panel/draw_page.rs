mod hud;
mod manual;
mod overlays;

use crate::state::{DataState, InteractionState};
use crate::task::BackgroundTask;

use super::super::geometry::PageDimensions;
use super::{draw_drag_ghosts, helpers};

use hud::{HUD_GAP, HUD_HEIGHT};

/// Returns `(hovered_slot, over_page, page_rect, cursor_mm)`.
pub(super) fn draw_page(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    page_idx: usize,
    cmds: &mut Vec<BackgroundTask>,
) -> (Option<usize>, bool, egui::Rect, (f32, f32)) {
    let (width_mm, height_mm) = data.project.page_dimensions_mm(page_idx);
    let (bleed_mm, margin_mm) = data.project.page_bleed_margin_mm(page_idx);
    let dims = PageDimensions {
        width_mm,
        height_mm,
        bleed_mm,
        margin_mm,
    };
    let page_size = helpers::page_display_size(interaction.viewport.zoom, dims);

    let page_rect = render_page_image(ui, data, page_idx, page_size);
    ui.add_space(HUD_GAP + HUD_HEIGHT);

    let block_bottom = page_rect.max.y + HUD_GAP + HUD_HEIGHT;
    let block_rect =
        egui::Rect::from_min_max(page_rect.min, egui::pos2(page_rect.max.x, block_bottom));
    let hovered = ui
        .ctx()
        .input(|i| i.pointer.latest_pos().map(|p| block_rect.contains(p)))
        .unwrap_or(false);

    let dt = ui.ctx().input(|i| i.unstable_dt).min(0.05);
    let anim = interaction.page_hud.entry(page_idx).or_default();
    if anim.advance(hovered, dt) {
        ui.ctx().request_repaint();
    }

    let Some(layout_page) = data.project.layout.get(page_idx) else {
        return (None, false, page_rect, (0.0, 0.0));
    };

    overlays::draw_slot_overlays(ui, page_rect, data, interaction, page_idx, dims);

    let (hovered_slot, over_page, cursor_mm) = hit_test_pointer(ui, page_rect, layout_page, dims);

    super::super::page_nav::draw_nav_selection_overlay(ui, interaction, page_idx, page_rect);
    overlays::draw_page_move_highlight(ui, interaction, page_idx, page_rect, over_page);
    overlays::draw_pool_drag_overlay(ui, interaction, page_idx, page_rect);
    draw_drag_ghosts::draw_drag_ghosts(ui, data, interaction, page_idx, page_rect, dims);

    use fotobuch::dto_models::PageMode;
    if layout_page.mode == PageMode::Manual {
        let full_w_mm = dims.width_mm + 2.0 * dims.bleed_mm;
        let pixel_per_mm = if full_w_mm > 0.0 {
            page_rect.width() as f64 / full_w_mm
        } else {
            1.0
        };
        manual::draw_manual_handles_and_overlay(
            ui,
            data,
            interaction,
            page_idx,
            page_rect,
            dims,
            pixel_per_mm,
        );
    }

    let (opacity, pill_w, actions_alpha, actions_offset) = {
        let a = &interaction.page_hud[&page_idx];
        (a.opacity, a.pill_width, a.actions_alpha, a.actions_offset)
    };

    let hud_rect = egui::Rect::from_min_size(
        egui::pos2(page_rect.min.x, page_rect.max.y + HUD_GAP),
        egui::vec2(page_rect.width(), HUD_HEIGHT),
    );
    hud::draw_hud(
        ui,
        hud_rect,
        page_idx,
        layout_page.mode,
        opacity,
        pill_w,
        actions_alpha,
        actions_offset,
        cmds,
    );

    // Register HUD for help lens mode.
    if ui.rect_contains_pointer(hud_rect) {
        interaction.help.hovered_widget = Some(("central-hud", hud_rect));
    }
    if interaction.help.highlighted == Some("central-hud") {
        let time = ui.ctx().input(|i| i.time);
        crate::app::help::draw_glow(ui.painter(), hud_rect, time);
    }

    (hovered_slot, over_page, page_rect, cursor_mm)
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

fn hit_test_pointer(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    layout_page: &fotobuch::dto_models::LayoutPage,
    dims: PageDimensions,
) -> (Option<usize>, bool, (f32, f32)) {
    use super::super::geometry;
    match ui.input(|i| i.pointer.latest_pos()) {
        None => (None, false, (0.0, 0.0)),
        Some(pos) => {
            let slot = geometry::hit_test_slot(pos, page_rect, layout_page, dims);
            let over = page_rect.contains(pos);
            let cursor_mm = if over {
                let s = dims.page_scale(page_rect);
                let x_mm = (pos.x - page_rect.min.x) / s.scale_x - s.offset_mm;
                let y_mm = (pos.y - page_rect.min.y) / s.scale_y - s.offset_mm;
                (x_mm, y_mm)
            } else {
                (0.0, 0.0)
            };
            (slot, over, cursor_mm)
        }
    }
}
