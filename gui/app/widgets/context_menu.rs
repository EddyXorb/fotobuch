use crate::state::{ContextMenu, DataState, InteractionState, WeightSlider};
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
        ContextMenu::Slot {
            page,
            slot,
            screen_pos,
            ..
        } => {
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
            if ui.button("Set weight…").clicked() {
                let initial = compute_slot_weight(data, *page, *slot);
                interaction.weight_slider = WeightSlider::Open {
                    page: *page,
                    slots: vec![*slot],
                    screen_pos: *screen_pos,
                    value: initial,
                };
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
            // If the right-clicked page is part of the current nav selection, operate on
            // all selected pages; otherwise fall back to just the right-clicked one.
            let target_pages: Vec<usize> = if interaction.selections.nav_pages.is_selected(page) {
                interaction.selections.nav_pages.items()
            } else {
                vec![*page]
            };

            if target_pages.len() == 1 {
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
            } else {
                if ui.button("Rebuild selected").clicked() {
                    cmds.push(BackgroundTask::RebuildPages {
                        pages: target_pages.clone(),
                    });
                    interaction.context_menu = None;
                }
            }

            let pages_to_delete: Vec<usize> = if data.project.has_cover() {
                target_pages.into_iter().filter(|&p| p != 0).collect()
            } else {
                target_pages
            };
            if !pages_to_delete.is_empty() && ui.button("Delete page").clicked() {
                cmds.push(BackgroundTask::DeletePages {
                    pages: pages_to_delete,
                });
                interaction.context_menu = None;
            }
        }
        ContextMenu::PoolItem { id, .. } => {
            // If the right-clicked photo is part of the current selection, operate on all
            // selected photos; otherwise fall back to just the clicked one.
            let target_ids: Vec<String> = if interaction.selections.photos.is_selected(id) {
                interaction.selections.photos.ids()
            } else {
                vec![id.clone()]
            };

            if ui.button("Remove").clicked() {
                cmds.push(BackgroundTask::RemovePhotos {
                    photo_ids: target_ids.clone(),
                });
                interaction.context_menu = None;
            }

            // Collect all placed (page, slot) pairs for the target photos, grouped by page.
            let mut by_page: std::collections::BTreeMap<usize, Vec<usize>> =
                std::collections::BTreeMap::new();
            for tid in &target_ids {
                if let Some(locs) = data.derived.placed_locations.get(tid.as_str()) {
                    for &(page, slot) in locs {
                        by_page.entry(page).or_default().push(slot);
                    }
                }
            }
            if !by_page.is_empty() && ui.button("Unplace").clicked() {
                for (page, slots) in by_page {
                    cmds.push(BackgroundTask::Unplace { page, slots });
                }
                interaction.context_menu = None;
            }
        }
    }
}

fn compute_slot_weight(data: &DataState, page: usize, slot: usize) -> f64 {
    let photo_id = data
        .project
        .layout
        .get(page)
        .and_then(|lp| lp.photos.get(slot));
    let Some(photo_id) = photo_id else {
        return 1.0;
    };
    data.project
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .find(|f| &f.id == photo_id)
        .map(|f| f.area_weight)
        .unwrap_or(1.0)
}
