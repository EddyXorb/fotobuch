pub(crate) mod draw_drag_ghosts;
mod draw_new_page_area;
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
    let resp = egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(FbTheme::BG))
        .show_inside(ui, |ui| {
            if let Some(h) = draw_pages::draw_pages(ui, data, interaction, cmds) {
                interaction.hovered = Some(h);
            }
            ui.max_rect()
        });
    let panel_rect = resp.inner;
    if ui.rect_contains_pointer(panel_rect) {
        interaction.help.hovered_widget = Some(("central-panel", panel_rect));
    }
    if interaction.help.highlighted == Some("central-panel") {
        let time = ui.ctx().input(|i| i.time);
        crate::app::help::draw_glow(ui.painter(), panel_rect, time);
    }
}
