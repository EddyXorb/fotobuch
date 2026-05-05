use crate::state::DataState;

/// Draw error toasts bottom-right. Returns `true` if any live toasts were shown.
pub fn show(ctx: &egui::Context, data: &mut DataState) -> bool {
    if !data.toasts.gc() {
        return false;
    }

    let screen = ctx.content_rect();
    const MARGIN: f32 = 12.0;
    const MAX_W: f32 = 360.0;
    const LINE_H: f32 = 28.0;
    let n = data.toasts.items.len();
    let total_h = n as f32 * LINE_H + (n.saturating_sub(1)) as f32 * 4.0 + 2.0 * MARGIN;
    let pos = egui::pos2(
        screen.max.x - MAX_W - MARGIN,
        screen.max.y - total_h - MARGIN,
    );

    egui::Area::new(egui::Id::new("error_toasts"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_W);
            for toast in &data.toasts.items {
                let elapsed = toast.shown_since.elapsed().as_secs_f32();
                let ttl = 6.0_f32;
                let alpha = ((1.0 - (elapsed / ttl).powi(2)) * 230.0).clamp(60.0, 230.0) as u8;
                let bg = egui::Color32::from_rgba_unmultiplied(180, 30, 30, alpha);
                let fg = egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha);
                egui::Frame::NONE
                    .fill(bg)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚠").color(fg));
                            ui.label(egui::RichText::new(&toast.message).color(fg).small());
                        });
                    });
                ui.add_space(4.0);
            }
        });

    // Request repaint while toasts are alive so they fade correctly.
    ctx.request_repaint();
    true
}
