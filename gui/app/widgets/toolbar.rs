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
        rebuild_button(ui, interaction, &mut cmds);
        release_button(ui, &mut cmds);
        history_buttons(ui, &mut cmds);
        config_button(ui, interaction);
        ui.separator();
        slot_info_checkbox(ui, data, &mut cmds);
        ui.separator();
        place_button(ui, interaction, &mut cmds);
        ui.separator();
        drag_mode_buttons(ui, interaction);
    });
    cmds
}

fn rebuild_button(
    ui: &mut egui::Ui,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    let label = match selected_pages_for_rebuild(interaction) {
        PagesForRebuild::Selected(ref pages) => format!("Rebuild ({})", pages.len()),
        PagesForRebuild::None => "Rebuild …".to_string(),
    };
    if ui.button(label).clicked() {
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
    if ui.button("Release").clicked() {
        cmds.push(BackgroundTask::ReleaseBuild);
    }
}

fn history_buttons(ui: &mut egui::Ui, cmds: &mut Vec<BackgroundTask>) {
    if ui.add(egui::Button::new("↩")).clicked() {
        cmds.push(BackgroundTask::Undo);
    }
    if ui.add(egui::Button::new("↪")).clicked() {
        cmds.push(BackgroundTask::Redo);
    }
}

fn config_button(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .add(egui::Button::selectable(
            interaction.config.open,
            "⚙ Config",
        ))
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

fn drag_mode_buttons(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    if ui
        .add(egui::Button::selectable(
            interaction.drag.mode == DragMode::Swap,
            "⇄ Swap",
        ))
        .clicked()
    {
        interaction.drag.mode = DragMode::Swap;
    }
    if ui
        .add(egui::Button::selectable(
            interaction.drag.mode == DragMode::Move,
            "→ Move",
        ))
        .clicked()
    {
        interaction.drag.mode = DragMode::Move;
    }
}

fn slot_info_checkbox(ui: &mut egui::Ui, data: &DataState, cmds: &mut Vec<BackgroundTask>) {
    let mut show = data.project.config.preview.show_slot_info;
    if ui.checkbox(&mut show, "Slot info").changed() {
        cmds.push(BackgroundTask::ConfigSet {
            key: "preview.show_slot_info".to_string(),
            value: show.to_string(),
        });
    }
}
