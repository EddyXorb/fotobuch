mod row;

use std::path::PathBuf;

use crate::state::{ActiveDrag, DataState, DragSource, InteractionState};

pub fn draw(ui: &mut egui::Ui, data: &DataState, interaction: &mut InteractionState) {
    egui::Panel::left("photo_pool")
        .resizable(true)
        .min_size(120.0)
        .default_size(260.0)
        .show_inside(ui, |ui| show(ui, data, interaction));
}

fn draw_pool_drag_ghost(ctx: &egui::Context, data: &DataState, interaction: &InteractionState) {
    const THUMB: f32 = 24.0;
    const GAP: f32 = 2.0;
    const OFFSET_X: f32 = 8.0;

    let photo_ids = match &interaction.drag.active {
        ActiveDrag::Dragging(DragSource::Pool { photo_ids }) => photo_ids,
        _ => return,
    };
    let cursor = match ctx.pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("pool_drag_ghost"),
    ));
    let n = photo_ids.len();
    let total_h = n as f32 * THUMB + (n.saturating_sub(1)) as f32 * GAP;
    let top_y = cursor.y - total_h / 2.0;
    let left_x = cursor.x + OFFSET_X;
    for (i, id) in photo_ids.iter().enumerate() {
        let y = top_y + i as f32 * (THUMB + GAP);
        let rect = egui::Rect::from_min_size(egui::pos2(left_x, y), egui::vec2(THUMB, THUMB));
        if let Some(tex) = data.thumbs.get(id) {
            painter.image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
            );
        } else {
            painter.rect_filled(
                rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(100, 149, 237, 120),
            );
        }
    }
}

fn show(ui: &mut egui::Ui, data: &DataState, interaction: &mut InteractionState) {
    // Collect only the strings needed for drawing before the mutable closure borrows state.
    let groups: Vec<(String, Vec<(String, PathBuf)>)> = data
        .project
        .photos
        .iter()
        .map(|g| {
            let files = g
                .files
                .iter()
                .map(|f| (f.id.clone(), PathBuf::from(&f.source)))
                .collect();
            (g.group.clone(), files)
        })
        .collect();
    let order: Vec<String> = groups
        .iter()
        .flat_map(|(_, files)| files.iter().map(|(id, _)| id.clone()))
        .collect();

    let rmbactive = ui.input(|i| {
        (i.pointer.secondary_down() || i.pointer.secondary_released()) && !i.pointer.primary_down()
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .content_margin(egui::Margin {
            right: ui.spacing().scroll.bar_width as i8,
            ..egui::Margin::ZERO
        })
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            drag: !rmbactive,
            scroll_bar: true,
            mouse_wheel: true,
        })
        .show(ui, |ui| {
            for (group_name, files) in &groups {
                let group_id = ui.make_persistent_id(("pool_group", group_name));
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    group_id,
                    true,
                )
                .show_header(ui, |ui| {
                    // Truncate the group name to the available width so a long name
                    // shows "…" instead of widening the panel. Full name on hover.
                    ui.add(egui::Label::new(group_name).truncate())
                        .on_hover_text(group_name);
                })
                .body(|ui| {
                    for (id, source) in files {
                        row::draw_row(ui, data, interaction, id, source, &order);
                    }
                });
            }
        });

    draw_pool_drag_ghost(ui.ctx(), data, interaction);

    let panel_rect = ui.max_rect();
    if ui.rect_contains_pointer(panel_rect) {
        interaction.help.hovered_widget = Some(("pool-panel", panel_rect));
    }
    if interaction.help.highlighted == Some("pool-panel") {
        let time = ui.ctx().input(|i| i.time);
        crate::app::help::draw_glow(ui.painter(), panel_rect, time);
    }
}

#[cfg(test)]
mod tests {
    /// Reproduces the photo-pool layout (left panel → vertical `ScrollArea` →
    /// `CollapsingState` with a long header label and a row) and returns the
    /// panel width after layout when the panel has been dragged to its minimum.
    ///
    /// In a row the truncating filename label fills all available width; the
    /// trailing placement badge is then placed past the panel edge, growing the
    /// content and making the panel "snap back" to a wider size after resizing.
    /// `apply_clamp` mirrors the fix: reserve the badge width before the label.
    fn measure_pool_width(apply_clamp: bool) -> f32 {
        use std::sync::{Arc, Mutex};

        const MIN: f32 = 120.0;
        let long_name = "VeryLongPhotoGroupName".repeat(10);
        let width = Arc::new(Mutex::new(0.0_f32));
        let out = width.clone();

        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .build_ui(move |ui| {
                let id = egui::Id::new("photo_pool");
                // Simulate the user having dragged the panel down to its minimum.
                ui.ctx().data_mut(|d| {
                    d.insert_persisted(
                        id,
                        egui::containers::panel::PanelState {
                            rect: egui::Rect::from_min_size(
                                egui::pos2(0.0, 0.0),
                                egui::vec2(MIN, 600.0),
                            ),
                        },
                    );
                });

                let resp = egui::Panel::left(id)
                    .resizable(true)
                    .min_size(MIN)
                    .default_size(260.0)
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    egui::Id::new("grp"),
                                    true,
                                )
                                .show_header(ui, |ui| {
                                    ui.add(egui::Label::new(&long_name).truncate());
                                })
                                .body(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.allocate_exact_size(
                                            egui::vec2(24.0, 24.0),
                                            egui::Sense::hover(),
                                        );
                                        let reserve = 16.0 + ui.spacing().item_spacing.x;
                                        if apply_clamp {
                                            let label_w = (ui.available_width() - reserve).max(0.0);
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(label_w, 24.0),
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.add(egui::Label::new(&long_name).truncate());
                                                },
                                            );
                                        } else {
                                            ui.add(egui::Label::new(&long_name).truncate());
                                        }
                                        ui.allocate_exact_size(
                                            egui::vec2(16.0, 16.0),
                                            egui::Sense::hover(),
                                        );
                                    });
                                });
                            });
                    });
                *out.lock().unwrap() = resp.response.rect.width();
            });

        harness.run();
        let w = *width.lock().unwrap();
        w
    }

    #[test]
    fn pool_panel_does_not_snap_back_to_a_wider_size() {
        // Without reserving room for the trailing badge, the truncating filename
        // label fills all available width and pushes the badge past the panel
        // edge, growing the panel (snap-back). With the reservation it stays put.
        let without_fix = measure_pool_width(false);
        let with_fix = measure_pool_width(true);
        assert!(
            without_fix > 130.0,
            "expected the unfixed layout to overflow past 120px, got {without_fix}"
        );
        assert!(
            with_fix <= 125.0,
            "panel should stay near its 120px minimum, got {with_fix}"
        );
    }
}
