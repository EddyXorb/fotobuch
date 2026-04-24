pub(crate) mod draw_drag_ghosts;
mod draw_page;
mod draw_pages;
mod helpers;

use crate::state::{DataState, InteractionState};

pub fn draw(ui: &mut egui::Ui, data: &DataState, interaction: &mut InteractionState) {
    egui::CentralPanel::default().show_inside(ui, |ui| {
        if let Some(h) = draw_pages::draw_pages(ui, data, interaction) {
            interaction.hovered = Some(h);
        }
    });
}
