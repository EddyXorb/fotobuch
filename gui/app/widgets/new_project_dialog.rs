use crate::state::{InteractionState, NewProjectDialogState};
use crate::task::BackgroundTask;
use fotobuch::commands::project::new::{NewConfig, validate_project_name};
use fotobuch::models::{PreviewConfig, ProjectConfig};

pub fn show(
    ctx: &egui::Context,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !interaction.new_project_dialog.open {
        return;
    }
    let mut open = interaction.new_project_dialog.open;
    egui::Window::new("New Project")
        .open(&mut open)
        .default_size([380.0, 420.0])
        .resizable(false)
        .show(ctx, |ui| {
            body(ui, &mut interaction.new_project_dialog, cmds);
        });

    interaction.new_project_dialog.open = open && interaction.new_project_dialog.open;
}

fn body(ui: &mut egui::Ui, s: &mut NewProjectDialogState, cmds: &mut Vec<BackgroundTask>) {
    egui::Grid::new("new_project_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut s.name);
            ui.end_row();

            ui.label("Width (mm)");
            ui.text_edit_singleline(&mut s.width_mm);
            ui.end_row();

            ui.label("Height (mm)");
            ui.text_edit_singleline(&mut s.height_mm);
            ui.end_row();

            ui.label("Bleed (mm)");
            ui.text_edit_singleline(&mut s.bleed_mm);
            ui.end_row();

            ui.label("Margin (mm)");
            ui.text_edit_singleline(&mut s.margin_mm);
            ui.end_row();

            ui.label("With cover");
            ui.checkbox(&mut s.with_cover, "");
            ui.end_row();
        });

    if s.with_cover {
        ui.separator();
        ui.label("Spine");
        egui::Grid::new("spine_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.radio_value(&mut s.spine_fixed, false, "Auto (mm per 10 pages)");
                ui.end_row();
                ui.radio_value(&mut s.spine_fixed, true, "Fixed (mm)");
                ui.end_row();
                ui.label("Value (mm)");
                ui.text_edit_singleline(&mut s.spine_value);
                ui.end_row();
            });
    }

    ui.separator();

    let config = parse_config(s);
    let name_valid = validate_project_name(&s.name).is_ok();
    let spine_valid =
        !s.with_cover || (!s.spine_value.is_empty() && s.spine_value.parse::<f64>().is_ok());
    let dims_valid = s.width_mm.parse::<f64>().is_ok()
        && s.height_mm.parse::<f64>().is_ok()
        && s.bleed_mm.parse::<f64>().is_ok()
        && s.margin_mm.parse::<f64>().is_ok();
    let can_submit = name_valid && dims_valid && spine_valid;

    ui.horizontal(|ui| {
        let btn = ui.add_enabled(can_submit, egui::Button::new("Create"));
        if btn.clicked()
            && let Some(cfg) = config
        {
            cmds.push(BackgroundTask::ProjectNew {
                config: Box::new(cfg),
            });
            cmds.push(BackgroundTask::ListProjects);
            s.open = false;
        }
        if !name_valid && !s.name.is_empty() {
            ui.label(
                egui::RichText::new("Invalid name")
                    .color(egui::Color32::from_rgb(220, 50, 50))
                    .small(),
            );
        };
    });
}

fn parse_config(s: &NewProjectDialogState) -> Option<NewConfig> {
    let width_mm = s.width_mm.parse::<f64>().ok()?;
    let height_mm = s.height_mm.parse::<f64>().ok()?;
    let bleed_mm = s.bleed_mm.parse::<f64>().ok()?;
    let margin_mm = s.margin_mm.parse::<f64>().ok()?;

    let (spine_grow_per_10_pages_mm, spine_mm) = if s.with_cover {
        let v = s.spine_value.parse::<f64>().ok()?;
        if s.spine_fixed {
            (None, Some(v))
        } else {
            (Some(v), None)
        }
    } else {
        (None, None)
    };

    // Preview overlays look cluttered in the GUI, so disable them by default.
    // The GUI renders pages directly, so writing the preview PDF on every build
    // only slows page rendering down — disable it by default.
    let base_config = ProjectConfig {
        preview: PreviewConfig {
            show_slot_info: false,
            show_preview_watermark: false,
            show_borders: false,
            show_filenames: false,
            write_pdf: false,
            ..Default::default()
        },
        ..Default::default()
    };

    Some(NewConfig {
        name: s.name.clone(),
        width_mm,
        height_mm,
        bleed_mm,
        with_cover: s.with_cover,
        cover_width_mm: None,
        cover_height_mm: None,
        spine_grow_per_10_pages_mm,
        spine_mm,
        margin_mm,
        base_config: Some(base_config),
    })
}
