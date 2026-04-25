use crate::state::{ActiveDrag, InteractionState};

pub(super) const HEIGHT_PT: f32 = 14.0;

pub(super) fn draw(
    ui: &mut egui::Ui,
    at_position: usize,
    interaction: &InteractionState,
) -> (egui::Rect, bool) {
    let desired = egui::vec2(ui.available_width(), HEIGHT_PT);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::hover());

    let drag_active = !matches!(interaction.drag.active, ActiveDrag::Idle);
    let hovered = resp.hovered();

    let alpha: u8 = match (drag_active, hovered) {
        (_, true) => 180,
        (true, false) => 80,
        _ => 20,
    };
    ui.painter().rect_filled(
        rect,
        3.0,
        egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha),
    );
    if hovered {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(HEIGHT_PT - 2.0),
            egui::Color32::WHITE,
        );
    }
    let _ = at_position;
    (rect, hovered)
}
