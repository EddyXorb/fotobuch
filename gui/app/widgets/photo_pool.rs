mod row;

use std::path::PathBuf;

use crate::state::{DataState, InteractionState};

pub fn draw(ui: &mut egui::Ui, data: &DataState, interaction: &mut InteractionState) {
    egui::Panel::left("photo_pool")
        .resizable(true)
        .min_size(220.0)
        .max_size(400.0)
        .default_size(260.0)
        .show_inside(ui, |ui| show(ui, data, interaction));
}

fn show(ui: &mut egui::Ui, data: &DataState, interaction: &mut InteractionState) {
    // Collect only the strings needed for drawing before the mutable closure borrows state.
    let groups: Vec<(String, Vec<(String, PathBuf)>)> = data
        .project
        .photos
        .iter()
        .map(|g| {
            let files = g
                .files
                .iter()
                .map(|f| (f.id.clone(), PathBuf::from(&f.source)))
                .collect();
            (g.group.clone(), files)
        })
        .collect();
    let order: Vec<String> = groups
        .iter()
        .flat_map(|(_, files)| files.iter().map(|(id, _)| id.clone()))
        .collect();

    let rmbactive = ui.input(|i| {
        (i.pointer.secondary_down() || i.pointer.secondary_released()) && !i.pointer.primary_down()
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .content_margin(egui::Margin {
            right: ui.spacing().scroll.bar_width as i8,
            ..egui::Margin::ZERO
        })
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            drag: !rmbactive,
            scroll_bar: true,
            mouse_wheel: true,
        })
        .show(ui, |ui| {
            for (group_name, files) in &groups {
                egui::CollapsingHeader::new(group_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        for (id, source) in files {
                            row::draw_row(ui, data, interaction, id, source, &order);
                        }
                    });
            }
        });
}
