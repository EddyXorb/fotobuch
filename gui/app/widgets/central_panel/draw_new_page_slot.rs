use fotobuch::commands::PlaceDst;

use crate::state::{ActiveDrag, DragSource, InteractionState};
use crate::task::BackgroundTask;

use super::theme::FbTheme;

const ZONE_HEIGHT: f32 = 44.0;
const ZONE_MARGIN: f32 = 14.0;

pub(super) fn draw(
    ui: &mut egui::Ui,
    at_position: usize,
    interaction: &InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) -> (egui::Rect, bool) {
    let row_desired = egui::vec2(ui.available_width(), ZONE_HEIGHT + ZONE_MARGIN * 2.0);
    let (row_rect, _) = ui.allocate_exact_size(row_desired, egui::Sense::hover());

    let zone_rect = egui::Rect::from_center_size(
        row_rect.center(),
        egui::vec2(row_rect.width().min(520.0), ZONE_HEIGHT),
    );

    let is_pool_drag = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Pool { .. })
    );
    let pointer_over = ui
        .ctx()
        .input(|i| i.pointer.latest_pos().map(|p| zone_rect.contains(p)))
        .unwrap_or(false);
    let active_and_hovered = is_pool_drag && pointer_over;

    // Animate transition toward accent when active+hovered.
    let anim_id = ui.id().with(("dropzone_anim", at_position));
    let t = ui
        .ctx()
        .animate_bool_with_time(anim_id, active_and_hovered, 0.15);

    let lerp_color = |a: egui::Color32, b: egui::Color32, f: f32| {
        egui::Color32::from_rgba_unmultiplied(
            (a.r() as f32 + (b.r() as f32 - a.r() as f32) * f) as u8,
            (a.g() as f32 + (b.g() as f32 - a.g() as f32) * f) as u8,
            (a.b() as f32 + (b.b() as f32 - a.b() as f32) * f) as u8,
            (a.a() as f32 + (b.a() as f32 - a.a() as f32) * f) as u8,
        )
    };

    let border_idle = FbTheme::STROKE;
    let border_active = FbTheme::ACCENT;
    let border_color = lerp_color(border_idle, border_active, t);

    let bg_idle = egui::Color32::TRANSPARENT;
    let bg_active = FbTheme::with_alpha(FbTheme::ACCENT, 0x14);
    let bg_color = lerp_color(bg_idle, bg_active, t);

    let painter = ui.painter();

    // Dashed border
    painter.rect_filled(zone_rect, 4.0, bg_color);
    draw_dashed_rect(painter, zone_rect, border_color, 1.5, 6.0, 4.0);

    // Label text
    let (label, text_color) = if active_and_hovered {
        ("Release to add new page", FbTheme::ACCENT)
    } else {
        ("Drop a photo here to add a new page", FbTheme::TEXT_MUTE)
    };
    let text_alpha = if is_pool_drag { 255 } else { 100 };
    painter.text(
        zone_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(10.5),
        FbTheme::with_alpha(text_color, text_alpha),
    );

    // Emit Place task on pointer release while drag-over.
    if active_and_hovered && ui.ctx().input(|i| i.pointer.any_released()) {
        if let ActiveDrag::Dragging(DragSource::Pool { photo_ids }) = &interaction.drag.active {
            cmds.push(BackgroundTask::Place {
                photo_ids: photo_ids.clone(),
                dst: PlaceDst::NewPageAt(at_position),
            });
        }
    }

    (zone_rect, pointer_over)
}

fn draw_dashed_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    thickness: f32,
    dash_len: f32,
    gap_len: f32,
) {
    let stroke = egui::Stroke::new(thickness, color);
    let corners = [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ];
    for (from, to) in corners {
        draw_dashed_line(painter, from, to, stroke, dash_len, gap_len);
    }
}

fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: egui::Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let total = from.distance(to);
    if total < 0.001 {
        return;
    }
    let dir = (to - from) / total;
    let step = dash_len + gap_len;
    let mut t = 0.0_f32;
    while t < total {
        let dash_end = (t + dash_len).min(total);
        painter.line_segment([from + dir * t, from + dir * dash_end], stroke);
        t += step;
    }
}
