use crate::state::{DataState, InteractionState};

pub fn show(ctx: &egui::Context, data: &DataState, interaction: &mut InteractionState) {
    if !interaction.show_history {
        return;
    }
    egui::Window::new("History")
        .default_size([400.0, 300.0])
        .show(ctx, |ui| {
            if data.history.is_empty() {
                ui.label("No history entries.");
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("history_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for entry in &data.history {
                            let date = entry.timestamp.format("%Y-%m-%d %H:%M").to_string();
                            ui.label(
                                egui::RichText::new(date)
                                    .monospace()
                                    .color(ui.visuals().weak_text_color()),
                            );
                            ui.label(&entry.message);
                            ui.end_row();
                        }
                    });
            });
        });
}
