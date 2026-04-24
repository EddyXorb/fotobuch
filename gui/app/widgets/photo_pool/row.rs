use std::path::Path;

use crate::state::{
    ActiveDrag, DataState, DragSource, HoveredTarget, InteractionState, PhotoSelection,
    SlotSelection,
};

pub(super) fn draw_row(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    id: &str,
    source: &Path,
    order: &[String],
) {
    let is_selected = interaction.selections.photos.is_selected(id);
    let is_pool_dragging = matches!(
        interaction.drag.active,
        ActiveDrag::Dragging(DragSource::Pool { .. })
    );

    let mut badge_hovered = false;
    let row_response = ui.push_id(egui::Id::new(("pool_row", id)), |ui| {
        let inner = ui.horizontal(|ui| {
            draw_thumb_cell(ui, data, id);
            draw_filename_label(ui, source, id);
            let badge_start_x = ui.cursor().min.x;
            badge_hovered = draw_placement_badge(ui, data, id);
            badge_start_x
        });
        let interact_rect = inner.response.rect.with_max_x(inner.inner);
        ui.interact(
            interact_rect,
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
        draw_hover_preview(ui, data, id, &row_resp);
    }

    if row_resp.hovered() {
        interaction.hovered = Some(HoveredTarget::PoolItem(id.to_string()));
    }

    handle_selection_click(ui, data, interaction, id, order, &row_resp);
}

fn draw_thumb_cell(ui: &mut egui::Ui, data: &DataState, id: &str) {
    let thumb_size = egui::vec2(24.0, 24.0);
    let (thumb_rect, _) = ui.allocate_exact_size(thumb_size, egui::Sense::hover());
    if let Some(tex) = data.thumbs.get(id) {
        ui.painter().image(
            tex.id(),
            thumb_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter()
            .rect_filled(thumb_rect, 0.0, egui::Color32::from_gray(160));
    }
}

fn draw_filename_label(ui: &mut egui::Ui, source: &Path, id: &str) {
    let filename = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| id.to_string());
    ui.add(egui::Label::new(&filename).truncate());
}

fn draw_placement_badge(ui: &mut egui::Ui, data: &DataState, id: &str) -> bool {
    let placed_count = data.derived.placed_count(id);
    let badge_color = match placed_count {
        0 => egui::Color32::TRANSPARENT,
        1 => egui::Color32::from_rgb(0, 200, 80),
        _ => egui::Color32::from_rgb(220, 40, 40),
    };
    let (badge_rect, badge_resp) =
        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(badge_rect.center(), 4.0, badge_color);

    if placed_count > 0 {
        let is_hovered = badge_resp.hovered();
        badge_resp.on_hover_ui(|ui| {
            if let Some(locs) = data.derived.placed_locations.get(id) {
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

fn draw_hover_preview(_ui: &mut egui::Ui, data: &DataState, id: &str, row_resp: &egui::Response) {
    if !row_resp.hovered() {
        return;
    }
    if let Some(tex) = data.thumbs.get(id) {
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
    data: &DataState,
    interaction: &mut InteractionState,
    id: &str,
    order: &[String],
    row_resp: &egui::Response,
) {
    if !row_resp.clicked() {
        return;
    }
    let mods = ui.input(|i| i.modifiers);
    if mods.shift {
        interaction
            .selections
            .photos
            .range_to(id.to_string(), order);
    } else if mods.ctrl || mods.command {
        interaction.selections.photos.toggle(id.to_string());
    } else {
        interaction.selections.photos = PhotoSelection::single(id.to_string());
        if let Some(locs) = data.derived.placed_locations.get(id)
            && locs.len() == 1
        {
            let (page, slot) = locs[0];
            interaction.viewport.scroll_to_page = Some(page);
            interaction.selections.slots = SlotSelection::single(page, slot);
        }
    }
}
