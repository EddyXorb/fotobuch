use crate::task::BackgroundTask;
use std::collections::HashSet;

use crate::state::InteractionState;

pub fn show(
    ctx: &egui::Context,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !interaction.rebuild_all_confirm {
        return;
    }
    let mut open = true;
    egui::Window::new("Rebuild all pages?")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("No page selected — rebuild will re-solve all pages.");
            ui.label("This may take several minutes depending on project size.");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    interaction.rebuild_all_confirm = false;
                }
                if ui.button("Rebuild all").clicked() {
                    cmds.push(BackgroundTask::RebuildAll);
                    interaction.rebuild_all_confirm = false;
                }
            });
        });
    if !open {
        interaction.rebuild_all_confirm = false;
    }
}
