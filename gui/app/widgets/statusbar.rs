use crate::state::{DragState, GuiState, HoveredTarget, Selection};

pub fn draw(ui: &mut egui::Ui, state: &GuiState) {
    egui::Panel::bottom("statusbar").show_inside(ui, |ui| show(ui, state));
}

fn show(ui: &mut egui::Ui, state: &GuiState) {
    ui.horizontal(|ui| {
        let total = state.project_state.layout.len();
        let photos: usize = state
            .project_state
            .photos
            .iter()
            .map(|g| g.files.len())
            .sum();
        let unplaced = state.derived.unplaced_photos.len();

        let page_str = match state.hovered.as_ref().and_then(HoveredTarget::slot) {
            Some((page, _)) => format!("Page {page}/{total}"),
            None => format!("Page \u{2013}/{total}"),
        };

        let sel_str = match &state.selection {
            Selection::None => "Sel: \u{2013}".to_string(),
            Selection::OnPage { page, slots, .. } => {
                format!("Sel: {} on page {page}", slots.len())
            }
        };

        let mode_str = state.drag_mode.label();
        let mode_display = if matches!(state.drag, DragState::Idle) {
            mode_str.to_string()
        } else {
            format!("[{}]", mode_str)
        };

        ui.label(format!(
            "{page_str} \u{b7} {photos} photos \u{b7} {unplaced} unplaced \u{b7} {sel_str} \u{b7} {mode_display}"
        ));
    });
}
