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
