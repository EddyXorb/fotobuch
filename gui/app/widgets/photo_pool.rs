use std::collections::HashSet;
use std::path::PathBuf;

use crate::app::pending::PendingCommand;
use crate::state::{GuiState, PoolDragState, PoolSelection};
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

fn show(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    // Build ordered list of all photo IDs for range selection.
    let order: Vec<String> = state
        .project_state
        .photos
        .iter()
        .flat_map(|g| g.files.iter().map(|f| f.id.clone()))
        .collect();

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
                                cmds,
                                &file.id,
                                &PathBuf::from(&file.source),
                                &order,
                                &mut visible_needed,
                            );
                        }
                    });
            }
        });

    // Dispatch thumbnail loading: visible-first, then prefetch.
    dispatch_thumb_loads(state, cmds, visible_needed);
}

fn draw_row(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    _cmds: &mut HashSet<PendingCommand>,
    id: &str,
    source: &PathBuf,
    order: &[String],
    visible_needed: &mut Vec<String>,
) {
    let is_selected = state.pool_selection.is_selected(id);
    let placed_count = state.derived.placed_count(id);

    let row_response = ui.push_id(egui::Id::new(("pool_row", id)), |ui| {
        ui.horizontal(|ui| {
            // 24×24 thumbnail cell
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
                // Mark as needed for this frame
                if !state.photo_thumbs.contains_key(id) && !state.photo_thumb_in_flight.contains(id)
                {
                    visible_needed.push(id.to_string());
                }
            }

            // Filename label (truncated)
            let filename = source
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| id.to_string());
            ui.add(egui::Label::new(&filename).truncate());

            // Badge: placed count indicator
            let badge_color = match placed_count {
                0 => egui::Color32::TRANSPARENT,
                1 => egui::Color32::from_rgb(0, 200, 80),
                _ => egui::Color32::from_rgb(220, 40, 40),
            };
            let badge_size = egui::vec2(8.0, 8.0);
            let (badge_rect, badge_resp) = ui.allocate_exact_size(badge_size, egui::Sense::hover());
            ui.painter()
                .circle_filled(badge_rect.center(), 4.0, badge_color);

            // Tooltip: list all placements
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
        })
        .response
    });

    let row_resp = row_response.inner;

    // Selection highlight background
    if is_selected {
        ui.painter().rect_filled(
            row_resp.rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(50, 120, 255, 40),
        );
    }

    // Hover lupe (256px preview)
    if row_resp.hovered() {
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

    // Click handling (selection)
    if row_resp.clicked() {
        let mods = ui.input(|i| i.modifiers);
        if mods.shift {
            state.pool_selection.range_to(id.to_string(), order);
        } else if mods.ctrl || mods.command {
            state.pool_selection.toggle(id.to_string());
        } else {
            state.pool_selection = PoolSelection::single(id.to_string());
        }
    }

    // Drag start (right mouse button)
    if row_resp.secondary_clicked() && matches!(state.pool_drag, PoolDragState::Idle) {
        let ids = if state.pool_selection.is_selected(id) {
            state.pool_selection.ids()
        } else {
            vec![id.to_string()]
        };
        state.pool_drag = PoolDragState::Dragging { photo_ids: ids };
    }

    // Pool drag in progress: show active drag ghost text near cursor
    if matches!(state.pool_drag, PoolDragState::Dragging { .. }) && is_selected {
        ui.painter().rect_filled(
            row_resp.rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(255, 200, 0, 30),
        );
    }
}

/// Dispatches thumbnail loading tasks: visible items first, then prefetch.
pub fn dispatch_thumb_loads(
    state: &mut GuiState,
    _cmds: &mut HashSet<PendingCommand>,
    visible_needed: Vec<String>,
) {
    // deduplicate visible_needed
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

    // deduplicate
    let mut seen = HashSet::new();
    visible_filtered.retain(|(id, _)| seen.insert(id.clone()));

    if !visible_filtered.is_empty() {
        for (id, _) in &visible_filtered {
            state.photo_thumb_in_flight.insert(id.clone());
        }
        // We can't send tasks from here directly (no task_tx) — caller must handle.
        // Store in state for pickup by FotobuchApp::dispatch_thumb_task.
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
