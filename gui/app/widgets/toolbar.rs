use crate::task::BackgroundTask;

use crate::app::rebuild::{PagesForRebuild, selected_pages_for_rebuild};
use crate::state::{DataState, DragMode, HoveredTarget, InteractionState};
use fotobuch::commands::PlaceDst;

pub fn draw(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
) -> Vec<BackgroundTask> {
    egui::Panel::top("toolbar")
        .show_inside(ui, |ui| show(ui, data, interaction))
        .inner
}

fn show(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
) -> Vec<BackgroundTask> {
    let mut cmds = Vec::new();
    ui.horizontal(|ui| {
        project_dropdown(ui, data, interaction, &mut cmds);
        ui.separator();
        add_button(ui, interaction);
        place_button(ui, interaction, &mut cmds);
        rebuild_button(ui, interaction, &mut cmds);
        release_button(ui, &mut cmds);
        ui.separator();
        history_buttons(ui, interaction, &mut cmds);
        ui.separator();
        config_button(ui, interaction);
        ui.separator();
        slot_info_checkbox(ui, data, &mut cmds);
        ui.separator();
        drag_mode_buttons(ui, interaction);
    });
    cmds
}

fn project_dropdown(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    let current_name = data
        .projects
        .iter()
        .find(|p| p.is_current)
        .map(|p| p.name.as_str())
        .unwrap_or("—");

    egui::ComboBox::from_id_salt("project_switcher")
        .selected_text(current_name)
        .show_ui(ui, |ui| {
            // Header: vault path
            ui.label(
                egui::RichText::new(data.vault_path.display().to_string())
                    .small()
                    .color(egui::Color32::GRAY),
            );
            ui.separator();

            // One entry per project
            for project in &data.projects {
                let label = if project.is_current {
                    format!("✓ {}", project.name)
                } else {
                    format!("  {}", project.name)
                };
                if ui.selectable_label(project.is_current, label).clicked() && !project.is_current {
                    cmds.push(BackgroundTask::ProjectSwitch {
                        name: project.name.clone(),
                    });
                    // Refresh project list after switch
                    cmds.push(BackgroundTask::ListProjects);
                }
            }

            ui.separator();
            if ui.button("+ New project …").clicked() {
                interaction.new_project_dialog.open = true;
                interaction.new_project_dialog.reset();
            }
            if ui.button("⇄ Switch vault …").clicked()
                && rfd::FileDialog::new().pick_folder().is_some()
            {
                interaction.toasts.push(
                    "Vault switch: restart with --vault <path> to open another vault.".to_string(),
                );
            }
        });
}

fn rebuild_button(
    ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    let label = match selected_pages_for_rebuild(interaction) {
        PagesForRebuild::Selected(ref pages) => format!("Rebuild ({})", pages.len()),
        PagesForRebuild::None => "Rebuild".to_string(),
    };
    if ui
        .button(label)
        .on_hover_text("Replan selected pages, or all pages if none selected [R]")
        .clicked()
    {
        match selected_pages_for_rebuild(interaction) {
            PagesForRebuild::Selected(pages) => {
                cmds.push(BackgroundTask::RebuildPages { pages });
            }
            PagesForRebuild::None => {
                interaction.rebuild_all_confirm = true;
            }
        }
    }
}

fn release_button(ui: &mut egui::Ui, cmds: &mut Vec<BackgroundTask>) {
    if ui
        .button("Release")
        .on_hover_text("Build the final release PDF [Ctrl+Shift+B]")
        .clicked()
    {
        cmds.push(BackgroundTask::ReleaseBuild);
    }
}

fn history_buttons(
    ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    if ui
        .add(egui::Button::new("↩"))
        .on_hover_text("Undo [Ctrl+Z]")
        .clicked()
    {
        cmds.push(BackgroundTask::Undo);
    }
    if ui
        .add(egui::Button::new("↪"))
        .on_hover_text("Redo [Ctrl+Y / Ctrl+Shift+Z]")
        .clicked()
    {
        cmds.push(BackgroundTask::Redo);
    }
    let hist_label = if interaction.show_history {
        "⏱ History ✓"
    } else {
        "⏱ History"
    };
    if ui
        .add(egui::Button::selectable(
            interaction.show_history,
            hist_label,
        ))
        .on_hover_text("Toggle commit history panel")
        .clicked()
    {
        interaction.show_history = !interaction.show_history;
        if interaction.show_history {
            cmds.push(BackgroundTask::LoadHistory { count: 100 });
        }
    }
}

fn config_button(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .add(egui::Button::selectable(
            interaction.config.open,
            "⚙ Config",
        ))
        .on_hover_text("Open project configuration panel [Ctrl+,]")
        .clicked()
    {
        interaction.config.open = !interaction.config.open;
    }
}

fn place_button(
    ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    let place_enabled = !interaction.selections.photos.is_empty();
    if ui
        .add_enabled(place_enabled, egui::Button::new("Place"))
        .on_hover_text("Place selected pool photos onto the hovered page, or auto-distribute [P]")
        .clicked()
    {
        cmds.push(BackgroundTask::Place {
            photo_ids: interaction.selections.photos.ids(),
            dst: match interaction
                .hovered
                .as_ref()
                .and_then(HoveredTarget::central_page)
            {
                Some(p) => PlaceDst::Page(p),
                None => PlaceDst::Auto,
            },
        });
    }
}

fn add_button(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .button("Add")
        .on_hover_text("Add photos from a folder to the pool [Ctrl+O]")
        .clicked()
    {
        interaction.add_dialog.open = true;
    }
}

fn drag_mode_buttons(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .add(egui::Button::selectable(
            interaction.drag.mode == DragMode::Swap,
            "⇄ Swap",
        ))
        .on_hover_text("RMB drag exchanges two slots [M toggles Swap/Move]")
        .clicked()
    {
        interaction.drag.mode = DragMode::Swap;
    }
    if ui
        .add(egui::Button::selectable(
            interaction.drag.mode == DragMode::Move,
            "→ Move",
        ))
        .on_hover_text("RMB drag moves slots to a new position or page [M toggles Swap/Move]")
        .clicked()
    {
        interaction.drag.mode = DragMode::Move;
    }
}

fn slot_info_checkbox(ui: &mut egui::Ui, data: &DataState, cmds: &mut Vec<BackgroundTask>) {
    let mut show = data.project.config.preview.show_slot_info;
    if ui
        .checkbox(&mut show, "Slot info")
        .on_hover_text("Overlay slot address and weight on each slot in the preview")
        .changed()
    {
        cmds.push(BackgroundTask::ConfigSet {
            key: "preview.show_slot_info".to_string(),
            value: show.to_string(),
        });
    }
}
