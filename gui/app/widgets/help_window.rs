use crate::app::help::{CHAPTERS, chips_for_chapter};
use crate::state::InteractionState;
use egui_commonmark::CommonMarkViewer;

pub fn show(ctx: &egui::Context, interaction: &mut InteractionState) {
    if !interaction.help.open {
        return;
    }

    // In lens mode, switch to the chapter matching the hovered widget.
    if interaction.help.lens_active
        && let Some((slug, _)) = interaction.help.hovered_widget
        && let Some(idx) = crate::app::help::chapter_for_slug(slug)
    {
        interaction.help.chapter = idx;
    }

    let mut open = interaction.help.open;
    egui::Window::new("Help")
        .open(&mut open)
        .resizable(true)
        .default_size([700.0, 500.0])
        .show(ctx, |ui| {
            draw_contents(ui, interaction);
        });
    interaction.help.open = open;
    if !open {
        interaction.help.highlighted = None;
    }
}

fn draw_contents(ui: &mut egui::Ui, interaction: &mut InteractionState) {
    ui.horizontal(|ui| {
        let lens_label = if interaction.help.lens_active {
            "● Lens ✓"
        } else {
            "● Lens"
        };
        if ui
            .add(egui::Button::selectable(
                interaction.help.lens_active,
                lens_label,
            ))
            .on_hover_text("Auto-switch chapter based on hovered widget")
            .clicked()
        {
            interaction.help.lens_active = !interaction.help.lens_active;
        }
    });
    ui.separator();

    ui.columns(2, |cols| {
        // Left column: sidebar chapter list
        egui::ScrollArea::vertical()
            .id_salt("help_sidebar")
            .show(&mut cols[0], |ui| {
                for (i, chapter) in CHAPTERS.iter().enumerate() {
                    let selected = interaction.help.chapter == i;
                    if ui.selectable_label(selected, chapter.title).clicked() {
                        interaction.help.chapter = i;
                        interaction.help.lens_active = false;
                    }
                }
            });

        // Right column: content + keyword chips
        egui::ScrollArea::vertical()
            .id_salt("help_content")
            .show(&mut cols[1], |ui| {
                let chapter_idx = interaction.help.chapter;
                if let Some(chapter) = CHAPTERS.get(chapter_idx) {
                    // Render markdown — borrow fields separately to avoid simultaneous borrows
                    let text = chapter.text;
                    let slug = chapter.slug;
                    CommonMarkViewer::new().show(ui, &mut interaction.help.cache, text);

                    // Related widget chips
                    let chips: Vec<(&'static str, &'static str)> =
                        chips_for_chapter(slug).collect();
                    if !chips.is_empty() {
                        ui.separator();
                        ui.label(egui::RichText::new("Related widgets:").small().weak());
                        ui.horizontal_wrapped(|ui| {
                            for (phrase, chip_slug) in chips {
                                let active = interaction.help.highlighted == Some(chip_slug);
                                let btn = egui::Button::selectable(active, phrase);
                                if ui.add(btn).clicked() {
                                    interaction.help.highlighted =
                                        if active { None } else { Some(chip_slug) };
                                }
                            }
                        });
                    }
                }
            });
    });
}
