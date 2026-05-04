use crate::state::{ActiveDrag, DragMode, InteractionState};

const SIZE_PT: f32 = 32.0;

pub(super) fn draw(
    ui: &mut egui::Ui,
    at_position: usize,
    interaction: &InteractionState,
) -> (egui::Rect, bool) {
    // Reserve full-width row, then center a square within it.
    let row_desired = egui::vec2(ui.available_width(), SIZE_PT + 8.0);
    let (row_rect, _) = ui.allocate_exact_size(row_desired, egui::Sense::hover());

    let center = row_rect.center();
    let half = SIZE_PT / 2.0;
    let sq = egui::Rect::from_center_size(center, egui::vec2(SIZE_PT, SIZE_PT));

    let drag_active = !matches!(interaction.drag.active, ActiveDrag::Idle);
    let is_swap = interaction.drag.mode == DragMode::Swap;
    // Use raw pointer position: resp.hovered() is unreliable during RMB drag.
    // In Swap mode, new-page-slot is not a valid drop target.
    let hovered = !is_swap
        && ui
            .ctx()
            .input(|i| i.pointer.latest_pos().map(|p| row_rect.contains(p)))
            .unwrap_or(false);

    let alpha: u8 = match (drag_active, hovered) {
        (true, true) => 200,
        (true, false) => 30,
        _ => 30,
    };
    let fill = egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha);
    let stroke_color =
        egui::Color32::from_rgba_unmultiplied(80, 160, 255, alpha.saturating_add(40));

    ui.painter().rect(
        sq,
        4.0,
        fill,
        egui::Stroke::new(1.5, stroke_color),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(half + 2.0),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha.saturating_add(55)),
    );

    let _ = at_position;
    (sq, hovered)
}
