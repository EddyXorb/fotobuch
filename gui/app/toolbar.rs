/// Toolbar with disabled action stubs (Phase 2 — no commands yet).
pub fn show(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_enabled(false, egui::Button::new("Build"));
        ui.add_enabled(false, egui::Button::new("Release"));
        ui.add_enabled(false, egui::Button::new("↩"));
        ui.add_enabled(false, egui::Button::new("↪"));
        ui.add_enabled(false, egui::Button::new("⚙"));
    });
}
