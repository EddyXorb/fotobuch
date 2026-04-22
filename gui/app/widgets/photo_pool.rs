use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::state::{DragSource, DragState, GuiState, HoveredTarget, PoolSelection, Selection};
use crate::task::BackgroundTask;

const THUMB_FILL_CHUNK: usize = 8;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) {
    egui::Panel::left("photo_pool")
        .resizable(true)
        .min_size(220.0)
        .max_size(400.0)
        .default_size(260.0)
        .show_inside(ui, |ui| show(ui, state));
}

fn show(ui: &mut egui::Ui, state: &mut GuiState) {
    // Collect only the strings needed for drawing before the mutable closure borrows state.
    let groups: Vec<(String, Vec<(String, PathBuf)>)> = state
        .project_state
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

    let mut visible_needed: Vec<String> = Vec::new();

    let rmbactive = ui.input(|i| {
        (i.pointer.secondary_down() || i.pointer.secondary_released()) && !i.pointer.primary_down()
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            drag: !rmbactive,
            scroll_bar: true,
            mouse_wheel: true,
        })
        .show(ui, |ui| {
            for (group_name, files) in &groups {
                egui::CollapsingHeader::new(group_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        for (id, source) in files {
                            draw_row(ui, state, id, source, &order, &mut visible_needed);
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

    let mut badge_hovered = false;
    let row_response = ui.push_id(egui::Id::new(("pool_row", id)), |ui| {
        let layout_resp = ui
            .horizontal(|ui| {
                draw_thumb_cell(ui, state, id, visible_needed);
                draw_filename_label(ui, source, id);
                badge_hovered = draw_placement_badge(ui, state, id);
            })
            .response;
        ui.interact(
            layout_resp.rect,
            egui::Id::new(("pool_row_interact", id)),
            egui::Sense::click_and_drag(),
        )
    });

    let row_resp = row_response.inner;

    if is_selected {
        draw_selection_highlight(ui, row_resp.rect);
    }
    if is_pool_dragging && is_selected {
        draw_drag_highlight(ui, row_resp.rect);
    }

    if !badge_hovered {
        draw_hover_preview(ui, state, id, &row_resp);
    }

    if row_resp.hovered() {
        state.hovered = Some(HoveredTarget::PoolItem(id.to_string()));
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
    if let Some(tex) = state.thumb.thumbs.get(id) {
        ui.painter().image(
            tex.id(),
            thumb_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter()
            .rect_filled(thumb_rect, 0.0, egui::Color32::from_gray(160));
        if needs_thumb_load(state, id) {
            visible_needed.push(id.to_string());
        }
    }
}

fn draw_filename_label(ui: &mut egui::Ui, source: &Path, id: &str) {
    let filename = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| id.to_string());
    ui.add(egui::Label::new(&filename).truncate());
}

fn draw_placement_badge(ui: &mut egui::Ui, state: &GuiState, id: &str) -> bool {
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
        let is_hovered = badge_resp.hovered();
        badge_resp.on_hover_ui(|ui| {
            if let Some(locs) = state.derived.placed_locations.get(id) {
                let mut sorted = locs.clone();
                sorted.sort();
                for (page, slot) in &sorted {
                    ui.label(format!("Page {page} Slot {slot}"));
                }
            }
        });
        return is_hovered;
    }
    false
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
    if let Some(tex) = state.thumb.thumbs.get(id) {
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
        if let Some(locs) = state.derived.placed_locations.get(id)
            && locs.len() == 1
        {
            let (page, slot) = locs[0];
            state.scroll_to_page = Some(page);
            state.selection = Selection::single(page, slot);
        }
    }
}

fn needs_thumb_load(state: &GuiState, id: &str) -> bool {
    !state.thumb.thumbs.contains_key(id) && !state.thumb.in_flight.contains(id)
}

/// Dispatches thumbnail loading tasks: visible items first, then prefetch.
pub fn dispatch_thumb_loads(state: &mut GuiState, visible_needed: Vec<String>) {
    let mut visible_filtered: Vec<(String, PathBuf)> = visible_needed
        .into_iter()
        .filter(|id| !state.thumb.thumbs.contains_key(id) && !state.thumb.in_flight.contains(id))
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
            state.thumb.in_flight.insert(id.clone());
        }
        state.thumb.pending_loads.extend(visible_filtered);
    } else if !state.thumb.prefetch.is_empty() {
        let chunk_len = THUMB_FILL_CHUNK.min(state.thumb.prefetch.len());
        let chunk: Vec<String> = state.thumb.prefetch.drain(..chunk_len).collect();
        let items: Vec<(String, PathBuf)> = chunk
            .into_iter()
            .filter(|id| needs_thumb_load(state, id))
            .filter_map(|id| {
                state
                    .derived
                    .photo_by_id
                    .get(&id)
                    .map(|f| (id, PathBuf::from(&f.source)))
            })
            .collect();
        for (id, _) in &items {
            state.thumb.in_flight.insert(id.clone());
        }
        state.thumb.pending_loads.extend(items);
    }
}

/// Flushes pending thumb loads as a single BackgroundTask.
pub fn flush_thumb_loads(state: &mut GuiState) -> Option<BackgroundTask> {
    if state.thumb.pending_loads.is_empty() {
        return None;
    }
    let items: Vec<(String, PathBuf)> = state.thumb.pending_loads.drain(..).collect();
    Some(BackgroundTask::LoadPhotoThumbnails { items })
}
