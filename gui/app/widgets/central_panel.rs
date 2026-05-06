pub(crate) mod draw_drag_ghosts;
mod draw_new_page_slot;
mod draw_page;
mod draw_pages;
mod helpers;
pub(crate) mod manual_resize;
pub(crate) mod theme;

use crate::state::{DataState, InteractionState};
use crate::task::BackgroundTask;

use theme::FbTheme;

pub fn draw(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(FbTheme::BG))
        .show_inside(ui, |ui| {
            if let Some(h) = draw_pages::draw_pages(ui, data, interaction, cmds) {
                interaction.hovered = Some(h);
            }
        });
}
