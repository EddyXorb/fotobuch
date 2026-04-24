use crate::state::{ActiveDrag, DataState, HoveredTarget, InteractionState, SlotSelection};

pub fn draw(ui: &mut egui::Ui, data: &DataState, interaction: &InteractionState) {
    egui::Panel::bottom("statusbar").show_inside(ui, |ui| show(ui, data, interaction));
}

fn show(ui: &mut egui::Ui, data: &DataState, interaction: &InteractionState) {
    ui.horizontal(|ui| {
        let total = data.project.layout.len();
        let photos: usize = data.project.photos.iter().map(|g| g.files.len()).sum();
        let unplaced = data.derived.unplaced_photos.len();

        let page_str = match interaction.hovered.as_ref().and_then(HoveredTarget::slot) {
            Some((page, _)) => format!("Page {page}/{total}"),
            None => format!("Page \u{2013}/{total}"),
        };

        let sel_str = match &interaction.selections.slots {
            SlotSelection::None => "Sel: \u{2013}".to_string(),
            SlotSelection::OnPage { page, slots, .. } => {
                format!("Sel: {} on page {page}", slots.len())
            }
        };

        let mode_str = interaction.drag.mode.label();
        let mode_display = if matches!(interaction.drag.active, ActiveDrag::Idle) {
            mode_str.to_string()
        } else {
            format!("[{}]", mode_str)
        };

        ui.label(format!(
            "{page_str} \u{b7} {photos} photos \u{b7} {unplaced} unplaced \u{b7} {sel_str} \u{b7} {mode_display}"
        ));
    });
}
