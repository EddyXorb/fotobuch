use crate::state::{ActiveDrag, DataState, DragMode, DragSource, InteractionState, PageHudAnim};
use crate::task::BackgroundTask;

use super::super::geometry::{self, PageDimensions};
use super::theme::FbTheme;
use super::{draw_drag_ghosts, helpers};

const HUD_HEIGHT: f32 = 24.0;
const HUD_GAP: f32 = 10.0;

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

    // Allocate the whole block (page + gap + HUD) to detect hover over the row.
    let block_size = egui::vec2(page_size.x, page_size.y + HUD_GAP + HUD_HEIGHT);
    let (block_rect, _) = ui.allocate_exact_size(block_size, egui::Sense::hover());
    // Use raw pointer for hover — same pattern as elsewhere in this panel.
    let hovered = ui
        .ctx()
        .input(|i| i.pointer.latest_pos().map(|p| block_rect.contains(p)))
        .unwrap_or(false);

    // Advance animation.
    let dt = ui.ctx().input(|i| i.unstable_dt).min(0.05);
    let anim = interaction
        .page_hud
        .entry(page_idx)
        .or_insert_with(PageHudAnim::default);
    let still_moving = anim.advance(hovered, dt);
    if still_moving {
        ui.ctx().request_repaint();
    }

    // Draw page image inside the block rect (top portion).
    let page_rect = egui::Rect::from_min_size(block_rect.min, page_size);
    let child_ui_rect = page_rect;
    let page_rect = render_page_image(ui, data, page_idx, child_ui_rect);

    if let Some(layout_page) = data.project.layout.get(page_idx) {
        draw_slot_overlays(ui, page_rect, data, interaction, page_idx, dims);
        let (hovered_slot, over_page, cursor_mm) =
            hit_test_pointer(ui, page_rect, layout_page, dims);
        super::super::page_nav::draw_nav_selection_overlay(ui, interaction, page_idx, page_rect);
        draw_page_move_highlight(ui, interaction, page_idx, page_rect, over_page);
        draw_pool_drag_overlay(ui, interaction, page_idx, page_rect);
        draw_drag_ghosts::draw_drag_ghosts(ui, data, interaction, page_idx, page_rect, dims);

        use fotobuch::dto_models::PageMode;
        if layout_page.mode == PageMode::Manual {
            let full_w_mm = dims.width_mm + 2.0 * dims.bleed_mm;
            let pixel_per_mm = if full_w_mm > 0.0 {
                page_rect.width() as f64 / full_w_mm
            } else {
                1.0
            };
            draw_manual_handles_and_overlay(
                ui,
                data,
                interaction,
                page_idx,
                page_rect,
                dims,
                pixel_per_mm,
            );
        }

        // Snapshot anim values (borrow ends after this block).
        let (opacity, pill_w, actions_alpha, actions_offset) = {
            let a = &interaction.page_hud[&page_idx];
            (a.opacity, a.pill_width, a.actions_alpha, a.actions_offset)
        };

        // HUD strip: placed in the bottom 24px of the block.
        let hud_top = block_rect.min.y + page_size.y + HUD_GAP;
        let hud_rect = egui::Rect::from_min_size(
            egui::pos2(block_rect.min.x, hud_top),
            egui::vec2(block_rect.width(), HUD_HEIGHT),
        );
        draw_hud(
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

        (hovered_slot, over_page, page_rect, cursor_mm)
    } else {
        (None, false, page_rect, (0.0, 0.0))
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_hud(
    ui: &mut egui::Ui,
    hud_rect: egui::Rect,
    page_idx: usize,
    mode: fotobuch::dto_models::PageMode,
    opacity: f32,
    pill_width: f32,
    actions_alpha: f32,
    actions_offset: f32,
    cmds: &mut Vec<BackgroundTask>,
) {
    use fotobuch::dto_models::PageMode;

    let center_y = hud_rect.center().y;

    // The dot/pill is anchored at the horizontal center of the HUD (below page center).
    // The label grows leftward from it; actions grow rightward — so the dot never jumps.
    let label_w: f32 = 28.0;
    let btn_size: f32 = 22.0;
    let gap: f32 = 10.0;
    let dot_center_x = hud_rect.center().x;

    let alpha_u8 = (opacity * 255.0).clamp(0.0, 255.0) as u8;

    // Page number label — always gap+half-pill to the left of dot center.
    let label_str = format!("{}", page_idx + 1);
    let label_center = egui::pos2(
        dot_center_x - pill_width / 2.0 - gap - label_w / 2.0,
        center_y,
    );
    ui.painter().text(
        label_center,
        egui::Align2::CENTER_CENTER,
        &label_str,
        egui::FontId::monospace(10.5),
        FbTheme::with_alpha(FbTheme::TEXT_MUTE, alpha_u8),
    );

    // Mode dot / pill — centered on dot_center_x at all animation stages.
    let mode_color = match mode {
        PageMode::Auto => FbTheme::AUTO,
        PageMode::Manual => FbTheme::MANUAL,
    };

    // Idle = 18 px circle, hover = 80 px pill; height is always 18 px so the
    // layout space claimed by allocate_rect never changes between frames.
    const PILL_H: f32 = 18.0;
    let pill_rect = egui::Rect::from_center_size(
        egui::pos2(dot_center_x, center_y),
        egui::vec2(pill_width, PILL_H),
    );
    // 0.0 = idle circle, 1.0 = fully open pill.
    let expand_frac = ((pill_width - 18.0) / (80.0 - 18.0)).clamp(0.0, 1.0);

    let lerp_u8 = |a: u8, b: u8, t: f32| (a as f32 + (b as f32 - a as f32) * t) as u8;

    // Fill: solid gray dot → translucent mode-colored pill.
    let fill_r = lerp_u8(FbTheme::TEXT_MUTE.r(), mode_color.r(), expand_frac);
    let fill_g = lerp_u8(FbTheme::TEXT_MUTE.g(), mode_color.g(), expand_frac);
    let fill_b = lerp_u8(FbTheme::TEXT_MUTE.b(), mode_color.b(), expand_frac);
    let fill_a = lerp_u8(alpha_u8, (alpha_u8 as f32 * 0.13) as u8, expand_frac);
    let fill_color = egui::Color32::from_rgba_unmultiplied(fill_r, fill_g, fill_b, fill_a);

    // Border: invisible at dot, mode-colored at pill.
    let stroke_a = (alpha_u8 as f32 * 0.40 * expand_frac) as u8;
    let stroke_color = egui::Color32::from_rgba_unmultiplied(
        mode_color.r(),
        mode_color.g(),
        mode_color.b(),
        stroke_a,
    );

    ui.painter().rect(
        pill_rect,
        PILL_H / 2.0,
        fill_color,
        egui::Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );

    // Text fades in after the pill is wide enough to show it.
    if expand_frac > 0.01 {
        let pill_label = match mode {
            PageMode::Auto => "✦ AUTO",
            PageMode::Manual => "✋ MANUAL",
        };
        ui.painter().text(
            pill_rect.center(),
            egui::Align2::CENTER_CENTER,
            pill_label,
            egui::FontId::monospace(10.0),
            FbTheme::with_alpha(mode_color, (expand_frac * alpha_u8 as f32) as u8),
        );
    }

    // Pill click → mode toggle (allocate interaction area over pill_rect)
    let pill_resp = ui.allocate_rect(pill_rect, egui::Sense::click());
    if pill_resp.clicked() {
        let new_mode = match mode {
            PageMode::Auto => PageMode::Manual,
            PageMode::Manual => PageMode::Auto,
        };
        cmds.push(BackgroundTask::SetPageMode {
            page: page_idx,
            mode: new_mode,
        });
    }

    // Action buttons (fade in with actions_alpha)
    if actions_alpha > 0.001 {
        let act_alpha = (actions_alpha * alpha_u8 as f32) as u8;
        let btn_x = dot_center_x + pill_width / 2.0 + gap + actions_offset;

        // ↻ Rebuild button
        let rebuild_rect = egui::Rect::from_min_size(
            egui::pos2(btn_x, center_y - btn_size / 2.0),
            egui::vec2(btn_size, btn_size),
        );
        ui.painter().rect(
            rebuild_rect,
            4.0,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, FbTheme::with_alpha(FbTheme::STROKE, act_alpha)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rebuild_rect.center(),
            egui::Align2::CENTER_CENTER,
            "↻",
            egui::FontId::proportional(14.0),
            FbTheme::with_alpha(FbTheme::TEXT_DIM, act_alpha),
        );

        let rebuild_resp = ui.allocate_rect(rebuild_rect, egui::Sense::click());
        if rebuild_resp.clicked() {
            cmds.push(BackgroundTask::RebuildPages {
                pages: vec![page_idx],
            });
        }

        // ✕ Delete button
        let delete_rect = egui::Rect::from_min_size(
            egui::pos2(btn_x + btn_size + gap, center_y - btn_size / 2.0),
            egui::vec2(btn_size, btn_size),
        );
        ui.painter().rect(
            delete_rect,
            4.0,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, FbTheme::with_alpha(FbTheme::STROKE, act_alpha)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            delete_rect.center(),
            egui::Align2::CENTER_CENTER,
            "✕",
            egui::FontId::proportional(12.0),
            FbTheme::with_alpha(FbTheme::DANGER, act_alpha),
        );
        let delete_resp = ui.allocate_rect(delete_rect, egui::Sense::click());
        if delete_resp.clicked() {
            cmds.push(BackgroundTask::DeletePages {
                pages: vec![page_idx],
            });
        }
    }
}

fn render_page_image(
    ui: &mut egui::Ui,
    data: &DataState,
    page_idx: usize,
    page_rect: egui::Rect,
) -> egui::Rect {
    let size = page_rect.size();
    let rect = if let Some(tex) = &data.pages.textures[page_idx] {
        ui.put(
            page_rect,
            egui::Image::from_texture(tex).fit_to_exact_size(size),
        )
        .rect
    } else {
        ui.painter()
            .rect_filled(page_rect, 0.0, egui::Color32::from_gray(200));
        page_rect
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

    use fotobuch::dto_models::PageMode;
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
    }
}

fn hit_test_pointer(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    layout_page: &fotobuch::dto_models::LayoutPage,
    dims: PageDimensions,
) -> (Option<usize>, bool, (f32, f32)) {
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

fn se_corner_rect(slot_rect: egui::Rect) -> egui::Rect {
    const SZ: f32 = 8.0;
    egui::Rect::from_center_size(slot_rect.right_bottom(), egui::vec2(SZ, SZ))
}

#[allow(clippy::too_many_arguments)]
fn draw_manual_handles_and_overlay(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    page_idx: usize,
    page_rect: egui::Rect,
    dims: PageDimensions,
    pixel_per_mm: f64,
) {
    let layout_page = match data.project.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let cursor = ui.input(|i| i.pointer.hover_pos()).unwrap_or_default();
    let rmb_pressed = ui.input(|i| i.pointer.secondary_pressed());

    if rmb_pressed
        && interaction.drag.mode == DragMode::Move
        && matches!(interaction.drag.active, ActiveDrag::Idle)
    {
        for (slot_idx, slot) in layout_page.slots.iter().enumerate().rev() {
            let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
            let se = se_corner_rect(slot_rect);
            let source = if se.contains(cursor) {
                Some(DragSource::ManualResize {
                    page: page_idx,
                    slot: slot_idx,
                    pointer_origin: cursor,
                    slot_origin_mm: (slot.x_mm, slot.y_mm, slot.width_mm, slot.height_mm),
                    pixel_per_mm,
                })
            } else if slot_rect.contains(cursor) {
                Some(DragSource::ManualMove {
                    page: page_idx,
                    slot: slot_idx,
                    pointer_origin: cursor,
                    slot_origin_mm: (slot.x_mm, slot.y_mm),
                    pixel_per_mm,
                })
            } else {
                None
            };
            if let Some(src) = source {
                interaction.drag.active = ActiveDrag::Pending {
                    source: src,
                    press_pos: cursor,
                    press_instant: std::time::Instant::now(),
                };
                break;
            }
        }
    }

    let manual_dragging = matches!(
        &interaction.drag.active,
        ActiveDrag::Dragging(DragSource::ManualMove { page, .. } | DragSource::ManualResize { page, .. })
            if *page == page_idx
    );
    if !manual_dragging {
        for slot in &layout_page.slots {
            let slot_rect = geometry::slot_rect_on_screen(page_rect, dims, slot);
            let se = se_corner_rect(slot_rect);
            ui.painter().rect_filled(
                se,
                0.0,
                egui::Color32::from_rgba_unmultiplied(255, 200, 0, 200),
            );
        }
    }

    match &interaction.drag.active {
        ActiveDrag::Dragging(DragSource::ManualMove {
            page,
            slot,
            pointer_origin,
            slot_origin_mm,
            pixel_per_mm: ppm,
        }) if *page == page_idx => {
            let delta_px = cursor - *pointer_origin;
            let dx_mm = delta_px.x as f64 / ppm;
            let dy_mm = delta_px.y as f64 / ppm;
            if let Some(slot_data) = layout_page.slots.get(*slot) {
                let preview = fotobuch::dto_models::Slot {
                    x_mm: slot_origin_mm.0 + dx_mm,
                    y_mm: slot_origin_mm.1 + dy_mm,
                    width_mm: slot_data.width_mm,
                    height_mm: slot_data.height_mm,
                };
                let r = geometry::slot_rect_on_screen(page_rect, dims, &preview);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 128, 0)),
                    egui::StrokeKind::Outside,
                );
            }
        }
        ActiveDrag::Dragging(DragSource::ManualResize {
            page,
            slot,
            pointer_origin,
            slot_origin_mm,
            pixel_per_mm: ppm,
        }) if *page == page_idx => {
            let delta_px = cursor - *pointer_origin;
            let (_, _, new_w, new_h) =
                super::manual_resize::compute_se(*slot_origin_mm, delta_px, *ppm);
            if let Some(slot_data) = layout_page.slots.get(*slot) {
                let preview = fotobuch::dto_models::Slot {
                    x_mm: slot_origin_mm.0,
                    y_mm: slot_origin_mm.1,
                    width_mm: new_w,
                    height_mm: new_h,
                };
                let r = geometry::slot_rect_on_screen(page_rect, dims, &preview);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 255)),
                    egui::StrokeKind::Outside,
                );
                let _ = slot_data;
            }
            let _ = new_h;
        }
        _ => {}
    }
}
