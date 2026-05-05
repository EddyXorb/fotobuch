pub(crate) mod draw_drag_ghosts;
mod draw_new_page_slot;
mod draw_page;
mod draw_pages;
mod helpers;

use crate::state::{DataState, InteractionState};
use crate::task::BackgroundTask;

pub fn draw(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    egui::CentralPanel::default().show_inside(ui, |ui| {
        if let Some(h) = draw_pages::draw_pages(ui, data, interaction, cmds) {
            interaction.hovered = Some(h);
        }
    });
}
