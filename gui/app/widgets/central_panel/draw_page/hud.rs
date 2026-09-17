use crate::task::BackgroundTask;
use fotobuch::models::PageMode;

use super::super::theme::FbTheme;

pub(super) const HUD_HEIGHT: f32 = 20.0;
pub(super) const HUD_GAP: f32 = 10.0;

/// Geometrie- und Animationswerte eines HUD-Durchlaufs. Die Zeichenfunktionen
/// teilen sich diese Werte; als Einzelparameter waren es acht bis zwoelf.
#[derive(Clone, Copy)]
struct HudFrame {
    dot_center_x: f32,
    center_y: f32,
    pill_width: f32,
    gap: f32,
    alpha_u8: u8,
    actions_alpha: f32,
    actions_offset: f32,
}

/// `t` is the hover animation progress in `[0, 1]` (0 = idle, 1 = hovered).
pub(super) fn draw_hud(
    ui: &mut egui::Ui,
    hud_rect: egui::Rect,
    page_idx: usize,
    mode: PageMode,
    t: f32,
    cmds: &mut Vec<BackgroundTask>,
) {
    let lerp = |from: f32, to: f32| from + (to - from) * t;
    let opacity = lerp(0.55, 1.0);
    let pill_width = lerp(10.0, 80.0);
    let actions_alpha = t;
    let actions_offset = lerp(-4.0, 0.0);

    let center_y = hud_rect.center().y;
    let dot_center_x = hud_rect.center().x;
    let alpha_u8 = (opacity * 255.0).clamp(0.0, 255.0) as u8;

    const LABEL_W: f32 = 28.0;
    const BTN_SIZE: f32 = 22.0;
    const GAP: f32 = 10.0;

    let frame = HudFrame {
        dot_center_x,
        center_y,
        pill_width,
        gap: GAP,
        alpha_u8,
        actions_alpha,
        actions_offset,
    };

    draw_page_label(ui, &frame, LABEL_W, page_idx);

    let pill_rect = draw_mode_pill(ui, &frame, mode);

    let pointer = ui
        .ctx()
        .input(|i| i.pointer.interact_pos().filter(|_| i.pointer.any_click()));
    if pointer.is_some_and(|p| pill_rect.contains(p)) {
        let new_mode = match mode {
            PageMode::Auto => PageMode::Manual,
            PageMode::Manual => PageMode::Auto,
        };
        cmds.push(BackgroundTask::SetPageMode {
            page: page_idx,
            mode: new_mode,
        });
    }

    if actions_alpha > 0.001 {
        draw_action_buttons(ui, &frame, BTN_SIZE, page_idx, pointer, cmds);
    }
}

fn draw_page_label(ui: &egui::Ui, frame: &HudFrame, label_w: f32, page_idx: usize) {
    let HudFrame {
        dot_center_x,
        center_y,
        pill_width,
        gap,
        alpha_u8,
        ..
    } = *frame;

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
}

fn draw_mode_pill(ui: &egui::Ui, frame: &HudFrame, mode: PageMode) -> egui::Rect {
    let HudFrame {
        dot_center_x,
        center_y,
        pill_width,
        alpha_u8,
        ..
    } = *frame;

    let mode_color = match mode {
        PageMode::Auto => FbTheme::AUTO,
        PageMode::Manual => FbTheme::MANUAL,
    };

    let pill_height = pill_width.min(18.0);
    let corner_radius = (pill_width / 2.0).min(pill_height / 2.0);
    let pill_rect = egui::Rect::from_center_size(
        egui::pos2(dot_center_x, center_y),
        egui::vec2(pill_width, pill_height),
    );
    let expand_frac = ((pill_width - 10.0) / (80.0 - 10.0)).clamp(0.0, 1.0);

    let lerp_u8 = |a: u8, b: u8, t: f32| (a as f32 + (b as f32 - a as f32) * t) as u8;
    let fill_color = egui::Color32::from_rgba_unmultiplied(
        lerp_u8(FbTheme::TEXT_MUTE.r(), mode_color.r(), expand_frac),
        lerp_u8(FbTheme::TEXT_MUTE.g(), mode_color.g(), expand_frac),
        lerp_u8(FbTheme::TEXT_MUTE.b(), mode_color.b(), expand_frac),
        lerp_u8(alpha_u8, (alpha_u8 as f32 * 0.13) as u8, expand_frac),
    );
    let stroke_a = (alpha_u8 as f32 * 0.40 * expand_frac) as u8;
    let stroke_color = egui::Color32::from_rgba_unmultiplied(
        mode_color.r(),
        mode_color.g(),
        mode_color.b(),
        stroke_a,
    );

    ui.painter().rect(
        pill_rect,
        corner_radius,
        fill_color,
        egui::Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );

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

    pill_rect
}

fn draw_action_buttons(
    ui: &egui::Ui,
    frame: &HudFrame,
    btn_size: f32,
    page_idx: usize,
    pointer: Option<egui::Pos2>,
    cmds: &mut Vec<BackgroundTask>,
) {
    let HudFrame {
        dot_center_x,
        center_y,
        pill_width,
        gap,
        alpha_u8,
        actions_alpha,
        actions_offset,
    } = *frame;

    let act_alpha = (actions_alpha * alpha_u8 as f32) as u8;
    let btn_x = dot_center_x + pill_width / 2.0 + gap + actions_offset;

    let rebuild_rect = egui::Rect::from_min_size(
        egui::pos2(btn_x, center_y - btn_size / 2.0),
        egui::vec2(btn_size, btn_size),
    );
    draw_icon_button(ui, rebuild_rect, "↻", 14.0, FbTheme::TEXT_DIM, act_alpha);
    if pointer.is_some_and(|p| rebuild_rect.contains(p)) {
        cmds.push(BackgroundTask::RebuildPages {
            pages: vec![page_idx],
        });
    }

    let delete_rect = egui::Rect::from_min_size(
        egui::pos2(btn_x + btn_size + gap, center_y - btn_size / 2.0),
        egui::vec2(btn_size, btn_size),
    );
    draw_icon_button(ui, delete_rect, "✕", 12.0, FbTheme::DANGER, act_alpha);
    if pointer.is_some_and(|p| delete_rect.contains(p)) {
        cmds.push(BackgroundTask::DeletePages {
            pages: vec![page_idx],
        });
    }
}

fn draw_icon_button(
    ui: &egui::Ui,
    rect: egui::Rect,
    icon: &str,
    font_size: f32,
    icon_color: egui::Color32,
    alpha: u8,
) {
    ui.painter().rect(
        rect,
        4.0,
        egui::Color32::TRANSPARENT,
        egui::Stroke::new(1.0, FbTheme::with_alpha(FbTheme::STROKE, alpha)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(font_size),
        FbTheme::with_alpha(icon_color, alpha),
    );
}
