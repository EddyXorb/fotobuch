use crate::state::InteractionState;

pub fn show(ctx: &egui::Context, interaction: &mut InteractionState) {
    if !interaction.add_dialog_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Fotos hinzufügen")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Noch nicht implementiert.");
            if ui.button("Schließen").clicked() {
                interaction.add_dialog_open = false;
            }
        });
    if !open {
        interaction.add_dialog_open = false;
    }
}
