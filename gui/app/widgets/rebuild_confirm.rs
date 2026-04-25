use std::collections::HashSet;

use crate::app::pending::PendingCommand;
use crate::state::InteractionState;

pub fn show(
    ctx: &egui::Context,
    interaction: &mut InteractionState,
    cmds: &mut HashSet<PendingCommand>,
) {
    if !interaction.rebuild_all_confirm {
        return;
    }
    let mut open = true;
    egui::Window::new("Alle Seiten neu bauen?")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Keine Seite ausgewählt — der Rebuild würde alle Seiten neu solven.");
            ui.label("Das kann je nach Projektgröße mehrere Minuten dauern.");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Abbrechen").clicked() {
                    interaction.rebuild_all_confirm = false;
                }
                if ui.button("Alle neu bauen").clicked() {
                    cmds.insert(PendingCommand::RebuildAll);
                    interaction.rebuild_all_confirm = false;
                }
            });
        });
    if !open {
        interaction.rebuild_all_confirm = false;
    }
}
