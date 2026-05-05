use crate::state::InteractionState;
use crate::task::BackgroundTask;

pub fn show(
    ctx: &egui::Context,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !interaction.add_dialog.open {
        return;
    }
    let mut open = interaction.add_dialog.open;
    egui::Window::new("Add photos")
        .open(&mut open)
        .default_size([420.0, 320.0])
        .show(ctx, |ui| {
            if ui.button("Choose folder …").clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
            {
                interaction.add_dialog.pending_paths.push(dir);
            }
            ui.checkbox(&mut interaction.add_dialog.recursive, "Recursive");
            ui.horizontal(|ui| {
                ui.label("Weight:");
                ui.text_edit_singleline(&mut interaction.add_dialog.weight_buffer);
            });
            ui.horizontal(|ui| {
                ui.label("Path filter (regex):");
                ui.text_edit_singleline(&mut interaction.add_dialog.source_filter);
            });
            for p in &interaction.add_dialog.pending_paths {
                ui.label(p.display().to_string());
            }
            ui.separator();
            let can_submit = !interaction.add_dialog.pending_paths.is_empty();
            if ui
                .add_enabled(can_submit, egui::Button::new("Add"))
                .clicked()
            {
                let weight = interaction
                    .add_dialog
                    .weight_buffer
                    .parse::<f64>()
                    .unwrap_or(1.0);
                cmds.push(BackgroundTask::AddPhotos {
                    paths: std::mem::take(&mut interaction.add_dialog.pending_paths),
                    recursive: interaction.add_dialog.recursive,
                    weight,
                    source_filter: interaction.add_dialog.source_filter.clone(),
                });
                interaction.add_dialog.open = false;
            }
        });
    interaction.add_dialog.open = open && interaction.add_dialog.open;
}
