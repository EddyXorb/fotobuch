use crate::state::InteractionState;

pub fn show(ctx: &egui::Context, interaction: &mut InteractionState) {
    if !interaction.add_dialog_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Add photos")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Not yet implemented.");
            if ui.button("Close").clicked() {
                interaction.add_dialog_open = false;
            }
        });
    if !open {
        interaction.add_dialog_open = false;
    }
}
