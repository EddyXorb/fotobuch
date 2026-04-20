mod draw_drag_ghosts;
mod draw_page;
mod draw_pages;
mod helpers;

use crate::state::GuiState;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) {
    egui::CentralPanel::default().show_inside(ui, |ui| draw_pages::draw_pages(ui, state));
}
