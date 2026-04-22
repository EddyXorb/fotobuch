pub(crate) mod draw_drag_ghosts;
mod draw_page;
mod draw_pages;
mod helpers;

use crate::state::GuiState;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) {
    egui::CentralPanel::default().show_inside(ui, |ui| {
        let (hovered_slot, hovered_page) = draw_pages::draw_pages(ui, state);
        state.hovered_slot = hovered_slot;
        state.hovered_page = hovered_page;
    });
}
