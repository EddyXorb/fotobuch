use crate::state::{ContextMenu, DataState, InteractionState};
use crate::task::BackgroundTask;

pub fn show(
    ctx: &egui::Context,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    let menu = match interaction.context_menu.clone() {
        Some(m) => m,
        None => return,
    };

    let pos = match &menu {
        ContextMenu::Slot { screen_pos, .. } => *screen_pos,
        ContextMenu::Page { screen_pos, .. } => *screen_pos,
        ContextMenu::NavPage { screen_pos, .. } => *screen_pos,
        ContextMenu::PoolItem { screen_pos, .. } => *screen_pos,
    };

    let area_id = egui::Id::new("ctx_menu");
    let resp = egui::Area::new(area_id)
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                show_entries(ui, data, interaction, &menu, cmds);
            });
        });

    // Close on LMB press outside the menu area.
    let lmb_pressed = ctx.input(|i| i.pointer.primary_pressed());
    if lmb_pressed
        && !resp
            .response
            .rect
            .contains(ctx.pointer_interact_pos().unwrap_or_default())
    {
        interaction.context_menu = None;
    }
}

fn show_entries(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    menu: &ContextMenu,
    cmds: &mut Vec<BackgroundTask>,
) {
    match menu {
        ContextMenu::Slot { page, slot, .. } => {
            if ui.button("Unplace").clicked() {
                cmds.push(BackgroundTask::Unplace {
                    page: *page,
                    slots: vec![*slot],
                });
                interaction.context_menu = None;
            }
            if ui.button("Rebuild page").clicked() {
                cmds.push(BackgroundTask::RebuildPages { pages: vec![*page] });
                interaction.context_menu = None;
            }
        }
        ContextMenu::Page { page, .. } => {
            if ui.button("Rebuild").clicked() {
                cmds.push(BackgroundTask::RebuildPages { pages: vec![*page] });
                interaction.context_menu = None;
            }
            if let Some(lp) = data.project.layout.get(*page) {
                use fotobuch::dto_models::PageMode;
                let (label, new_mode) = match lp.mode {
                    PageMode::Auto => ("Set Manual", PageMode::Manual),
                    PageMode::Manual => ("Set Auto", PageMode::Auto),
                };
                if ui.button(label).clicked() {
                    cmds.push(BackgroundTask::SetPageMode {
                        page: *page,
                        mode: new_mode,
                    });
                    interaction.context_menu = None;
                }
            }
        }
        ContextMenu::NavPage { page, .. } => {
            if let Some(lp) = data.project.layout.get(*page) {
                use fotobuch::dto_models::PageMode;
                let (label, new_mode) = match lp.mode {
                    PageMode::Auto => ("Set Manual", PageMode::Manual),
                    PageMode::Manual => ("Set Auto", PageMode::Auto),
                };
                if ui.button(label).clicked() {
                    cmds.push(BackgroundTask::SetPageMode {
                        page: *page,
                        mode: new_mode,
                    });
                    interaction.context_menu = None;
                }
            }
            if ui.button("Rebuild").clicked() {
                cmds.push(BackgroundTask::RebuildPages { pages: vec![*page] });
                interaction.context_menu = None;
            }
            let is_cover = data.project.has_cover() && *page == 0;
            if !is_cover && ui.button("Delete page").clicked() {
                cmds.push(BackgroundTask::DeletePages { pages: vec![*page] });
                interaction.context_menu = None;
            }
        }
        ContextMenu::PoolItem { id, .. } => {
            if ui.button("Remove").clicked() {
                cmds.push(BackgroundTask::RemovePhotos {
                    photo_ids: vec![id.clone()],
                });
                interaction.context_menu = None;
            }
        }
    }
}
