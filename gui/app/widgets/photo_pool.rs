use std::collections::HashSet;
use std::path::PathBuf;

use crate::app::pending::PendingCommand;
use crate::state::{DragSource, DragState, GuiState, PoolSelection};
use crate::task::BackgroundTask;

pub const POOL_THUMB_MAX_EDGE_PX: u32 = 256;
const THUMB_FILL_CHUNK: usize = 8;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    egui::SidePanel::left("photo_pool")
        .resizable(true)
        .min_width(220.0)
        .max_width(400.0)
        .default_width(260.0)
        .show_inside(ui, |ui| show(ui, state, cmds));
}

fn show(ui: &mut egui::Ui, state: &mut GuiState, _cmds: &mut HashSet<PendingCommand>) {
    let order: Vec<String> = state
        .project_state
        .photos
        .iter()
        .flat_map(|g| g.files.iter().map(|f| f.id.clone()))
        .collect();

    state.hovered_pool_id = None;
    let mut visible_needed: Vec<String> = Vec::new();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for group in &state.project_state.photos.clone() {
                egui::CollapsingHeader::new(&group.group)
                    .default_open(true)
                    .show(ui, |ui| {
                        for file in &group.files {
                            draw_row(
                                ui,
                                state,
                                &file.id,
                                &PathBuf::from(&file.source),
                                &order,
                                &mut visible_needed,
                            );
                        }
                    });
            }
        });

    dispatch_thumb_loads(state, visible_needed);
}

fn draw_row(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    id: &str,
    source: &PathBuf,
    order: &[String],
    visible_needed: &mut Vec<String>,
) {
    let is_selected = state.pool_selection.is_selected(id);
    let is_pool_dragging = matches!(state.drag, DragState::Dragging(DragSource::Pool { .. }));

    let row_response = ui.push_id(egui::Id::new(("pool_row", id)), |ui| {
        ui.horizontal(|ui| {
            draw_thumb_cell(ui, state, id, visible_needed);
            draw_filename_label(ui, source, id);
            draw_placement_badge(ui, state, id);
        })
        .response
    });

    let row_resp = row_response.inner;

    if is_selected {
        draw_selection_highlight(ui, row_resp.rect);
    }
    if is_pool_dragging && is_selected {
        draw_drag_highlight(ui, row_resp.rect);
    }

    draw_hover_preview(ui, state, id, &row_resp);

    if row_resp.hovered() {
        state.hovered_pool_id = Some(id.to_string());
    }

    handle_selection_click(ui, state, id, order, &row_resp);
}

fn draw_thumb_cell(
    ui: &mut egui::Ui,
    state: &GuiState,
    id: &str,
    visible_needed: &mut Vec<String>,
) {
    let thumb_size = egui::vec2(24.0, 24.0);
    let (thumb_rect, _) = ui.allocate_exact_size(thumb_size, egui::Sense::hover());
    if let Some(tex) = state.photo_thumbs.get(id) {
        ui.painter().image(
            tex.id(),
            thumb_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter()
            .rect_filled(thumb_rect, 0.0, egui::Color32::from_gray(160));
        if !state.photo_thumbs.contains_key(id) && !state.photo_thumb_in_flight.contains(id) {
            visible_needed.push(id.to_string());
        }
    }
}

fn draw_filename_label(ui: &mut egui::Ui, source: &PathBuf, id: &str) {
    let filename = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| id.to_string());
    ui.add(egui::Label::new(&filename).truncate());
}

fn draw_placement_badge(ui: &mut egui::Ui, state: &GuiState, id: &str) {
    let placed_count = state.derived.placed_count(id);
    let badge_color = match placed_count {
        0 => egui::Color32::TRANSPARENT,
        1 => egui::Color32::from_rgb(0, 200, 80),
        _ => egui::Color32::from_rgb(220, 40, 40),
    };
    let (badge_rect, badge_resp) =
        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(badge_rect.center(), 4.0, badge_color);

    if placed_count > 0 {
        badge_resp.on_hover_ui(|ui| {
            if let Some(locs) = state.derived.placed_locations.get(id) {
                let mut sorted = locs.clone();
                sorted.sort();
                for (page, slot) in &sorted {
                    ui.label(format!("Page {page} Slot {slot}"));
                }
            }
        });
    }
}

fn draw_selection_highlight(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(50, 120, 255, 40),
    );
}

fn draw_drag_highlight(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(255, 200, 0, 30),
    );
}

fn draw_hover_preview(_ui: &mut egui::Ui, state: &GuiState, id: &str, row_resp: &egui::Response) {
    if !row_resp.hovered() {
        return;
    }
    if let Some(tex) = state.photo_thumbs.get(id) {
        let tex = tex.clone();
        row_resp.clone().on_hover_ui_at_pointer(|ui| {
            let sz = tex.size_vec2();
            ui.image((tex.id(), sz));
        });
    } else {
        row_resp.clone().on_hover_ui_at_pointer(|ui| {
            ui.spinner();
        });
    }
}

fn handle_selection_click(
    ui: &egui::Ui,
    state: &mut GuiState,
    id: &str,
    order: &[String],
    row_resp: &egui::Response,
) {
    if !row_resp.clicked() {
        return;
    }
    let mods = ui.input(|i| i.modifiers);
    if mods.shift {
        state.pool_selection.range_to(id.to_string(), order);
    } else if mods.ctrl || mods.command {
        state.pool_selection.toggle(id.to_string());
    } else {
        state.pool_selection = PoolSelection::single(id.to_string());
    }
}

/// Dispatches thumbnail loading tasks: visible items first, then prefetch.
pub fn dispatch_thumb_loads(state: &mut GuiState, visible_needed: Vec<String>) {
    let mut visible_filtered: Vec<(String, PathBuf)> = visible_needed
        .into_iter()
        .filter(|id| {
            !state.photo_thumbs.contains_key(id) && !state.photo_thumb_in_flight.contains(id)
        })
        .filter_map(|id| {
            state
                .derived
                .photo_by_id
                .get(&id)
                .map(|f| (id, PathBuf::from(&f.source)))
        })
        .collect();

    let mut seen = HashSet::new();
    visible_filtered.retain(|(id, _)| seen.insert(id.clone()));

    if !visible_filtered.is_empty() {
        for (id, _) in &visible_filtered {
            state.photo_thumb_in_flight.insert(id.clone());
        }
        state.pending_thumb_loads.extend(visible_filtered);
    } else if !state.photo_thumb_prefetch.is_empty() {
        let chunk_len = THUMB_FILL_CHUNK.min(state.photo_thumb_prefetch.len());
        let chunk: Vec<String> = state.photo_thumb_prefetch.drain(..chunk_len).collect();
        let items: Vec<(String, PathBuf)> = chunk
            .into_iter()
            .filter(|id| {
                !state.photo_thumbs.contains_key(id) && !state.photo_thumb_in_flight.contains(id)
            })
            .filter_map(|id| {
                state
                    .derived
                    .photo_by_id
                    .get(&id)
                    .map(|f| (id, PathBuf::from(&f.source)))
            })
            .collect();
        for (id, _) in &items {
            state.photo_thumb_in_flight.insert(id.clone());
        }
        state.pending_thumb_loads.extend(items);
    }
}

/// Flushes pending thumb loads as a single BackgroundTask.
pub fn flush_thumb_loads(state: &mut GuiState) -> Option<BackgroundTask> {
    if state.pending_thumb_loads.is_empty() {
        return None;
    }
    let items: Vec<(String, PathBuf)> = state.pending_thumb_loads.drain(..).collect();
    Some(BackgroundTask::LoadPhotoThumbnails { items })
}
