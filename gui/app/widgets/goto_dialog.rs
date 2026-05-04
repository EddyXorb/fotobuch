use crate::state::InteractionState;

pub fn show(ctx: &egui::Context, interaction: &mut InteractionState, num_pages: usize) {
    if !interaction.goto_open {
        return;
    }
    let mut open = interaction.goto_open;
    egui::Window::new("Go to page")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let id = egui::Id::new("goto_buffer");
            let mut buffer = ui.memory(|m| m.data.get_temp::<String>(id).unwrap_or_default());
            let resp = ui.text_edit_singleline(&mut buffer);
            ui.memory_mut(|m| m.data.insert_temp(id, buffer.clone()));

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                interaction.goto_open = false;
                ui.memory_mut(|m| m.data.remove::<String>(id));
                return;
            }

            let parsed = buffer
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|&p| p < num_pages);

            if resp.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && let Some(idx) = parsed
            {
                interaction.viewport.scroll_to_page = Some(idx);
                interaction.goto_open = false;
                ui.memory_mut(|m| m.data.remove::<String>(id));
            }

            if parsed.is_none() && !buffer.is_empty() {
                ui.colored_label(egui::Color32::LIGHT_RED, "Invalid page");
            }
        });
    interaction.goto_open = open && interaction.goto_open;
}
