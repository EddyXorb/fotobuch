use crate::state::InteractionState;
use crate::task::BackgroundTask;

/// First-run welcome modal.
///
/// Shown when no projects exist in the vault yet.  The user can either
/// create a new project or open an existing folder (which becomes the vault).
pub fn show(
    ctx: &egui::Context,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !interaction.show_welcome {
        return;
    }

    egui::Window::new("fotobuch")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.vertical_centered(|ui| {
                ui.heading("Welcome to fotobuch");
                ui.add_space(12.0);

                if ui
                    .button("  Create your first photobook  ")
                    .on_hover_text("Opens the new project dialog")
                    .clicked()
                {
                    interaction.new_project_dialog.open = true;
                    interaction.new_project_dialog.reset();
                }

                ui.add_space(6.0);

                if ui
                    .button("  Open an existing folder  ")
                    .on_hover_text("Open a folder that already contains a fotobuch vault")
                    .clicked()
                {
                    interaction.toasts.push(
                        "To open an existing vault, restart with: fotobuch-gui --vault <path>"
                            .to_string(),
                    );
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);

                ui.collapsing("Advanced", |ui| {
                    ui.label("Vault location is resolved in this priority:");
                    ui.label("  1. --vault <path> argument");
                    ui.label("  2. FOTOBUCH_VAULT environment variable");
                    ui.label("  3. Current directory (if it has fotobuch projects)");
                    ui.label("  4. Last opened vault from settings");
                    ui.label("  5. ~/Pictures/Fotobuch (default)");
                });
            });
        });

    // Suppress unused `cmds` warning — future: vault switch could push tasks
    let _ = cmds;
}
