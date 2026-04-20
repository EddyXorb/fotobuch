use egui::vec2;

use crate::state::{DragMode, DragState, GuiState, Selection};

use super::geometry;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState) {
    egui::CentralPanel::default().show_inside(ui, |ui| draw_pages(ui, state));
}

fn draw_page(ui: &mut egui::Ui, state: &GuiState, page_idx: usize) -> (Option<usize>, bool) {
    ui.label(format!("Page {page_idx}"));

    let (page_width_mm, page_height_mm) = state.project_state.page_dimensions_mm(page_idx);
    let size = page_display_size(state.zoom, page_width_mm, page_height_mm);
    let page_rect = render_page_image(ui, state, page_idx, size);

    if let Some(layout_page) = state.project_state.layout.get(page_idx) {
        draw_slot_overlays(
            ui,
            page_rect,
            state,
            page_idx,
            page_width_mm,
            page_height_mm,
        );
        let (hovered_slot, over_page) =
            hit_test_pointer(ui, page_rect, layout_page, page_width_mm, page_height_mm);
        draw_page_move_highlight(ui, state, page_idx, page_rect, over_page);
        draw_drag_ghosts(
            ui,
            state,
            page_idx,
            page_rect,
            page_width_mm,
            page_height_mm,
        );
        (hovered_slot, over_page)
    } else {
        (None, false)
    }
}

fn page_display_size(zoom: f32, page_width_mm: f64, page_height_mm: f64) -> egui::Vec2 {
    let mm_to_pt = 72.0_f32 / 25.4_f32;
    vec2(
        page_width_mm as f32 * mm_to_pt * zoom,
        page_height_mm as f32 * mm_to_pt * zoom,
    )
}

fn render_page_image(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    size: egui::Vec2,
) -> egui::Rect {
    let rect = if let Some(tex) = &state.page_textures[page_idx] {
        ui.add(egui::Image::from_texture(tex).fit_to_exact_size(size))
            .rect
    } else {
        let (r, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(r, 0.0, egui::Color32::from_gray(200));
        r
    };

    if state.page_dirty.get(page_idx).copied().unwrap_or(false) {
        ui.painter().rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(200, 200, 200, 150),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "↻",
            egui::FontId::proportional(rect.height() * 0.15),
            egui::Color32::from_gray(80),
        );
    }

    rect
}

fn draw_slot_overlays(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    state: &GuiState,
    page_idx: usize,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let drag_src_ratio: Option<f64> = if let DragState::Dragging {
        src_page, src_slot, ..
    } = &state.drag
    {
        if *src_page == page_idx {
            layout_page
                .slots
                .get(*src_slot)
                .map(|s| s.width_mm / s.height_mm)
        } else {
            None
        }
    } else {
        None
    };

    let is_dragging = !matches!(state.drag, DragState::Idle);
    let is_swap_drag = is_dragging && state.drag_mode == DragMode::Swap;

    let painter = ui.painter();
    for (slot_idx, slot) in layout_page.slots.iter().enumerate() {
        let slot_rect =
            geometry::slot_rect_on_screen(page_rect, page_width_mm, page_height_mm, slot);

        let is_hovered = state.hovered_slot == Some((page_idx, slot_idx));

        if is_swap_drag {
            let target_ratio = slot.width_mm / slot.height_mm;
            let same_ratio =
                drag_src_ratio.is_some_and(|r| geometry::slot_ratio_similar(r, target_ratio));
            let color = if same_ratio {
                egui::Color32::from_rgba_unmultiplied(0, 200, 80, 140)
            } else {
                egui::Color32::from_rgba_unmultiplied(220, 50, 50, 140)
            };
            painter.rect_filled(slot_rect, 0.0, color);
        } else if is_hovered && !is_dragging {
            painter.rect_filled(
                slot_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 120, 255, 38),
            );
        }

        if state.selection.is_selected(page_idx, slot_idx) {
            painter.rect_stroke(
                slot_rect,
                0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 200, 80)),
                egui::StrokeKind::Middle,
            );
        }
    }
}

fn draw_drag_ghosts(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let (src_slot_idx, cursor_at_drag_start) = match &state.drag {
        DragState::Dragging {
            src_page,
            src_slot,
            cursor_at_drag_start,
        } if *src_page == page_idx => (*src_slot, *cursor_at_drag_start),
        _ => return,
    };
    let cursor = match ui.ctx().pointer_hover_pos() {
        Some(p) => p,
        None => return,
    };
    let layout_page = match state.project_state.layout.get(page_idx) {
        Some(lp) => lp,
        None => return,
    };

    let scale_x = page_rect.width() / page_width_mm as f32;
    let scale_y = page_rect.height() / page_height_mm as f32;
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("drag_ghosts"),
    ));

    if let Some(slot) = layout_page.slots.get(src_slot_idx) {
        let rect = calc_primary_ghost_rect(
            page_rect,
            scale_x,
            scale_y,
            slot,
            cursor,
            cursor_at_drag_start,
        );
        paint_ghost_rect(&painter, rect, 120);
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)),
            egui::StrokeKind::Middle,
        );
        painter.text(
            rect.right_bottom() + vec2(6.0, -2.0),
            egui::Align2::LEFT_BOTTOM,
            state.drag_mode.label(),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }

    if state.drag_mode == DragMode::Swap {
        return;
    }

    let secondary: Vec<usize> = match &state.selection {
        Selection::OnPage { page, slots, .. } if *page == page_idx && slots.len() > 1 => slots
            .iter()
            .filter(|&&s| s != src_slot_idx)
            .copied()
            .collect(),
        _ => return,
    };

    let rects: Vec<egui::Rect> = secondary
        .iter()
        .filter_map(|&idx| {
            let slot = layout_page.slots.get(idx)?;
            Some(geometry::slot_rect_on_screen(
                page_rect,
                page_width_mm,
                page_height_mm,
                slot,
            ))
        })
        .collect();

    let max_dist = rects
        .iter()
        .map(|r| r.center().distance(cursor))
        .fold(0.0f32, f32::max)
        .max(1.0);

    for rect in rects {
        let t = (rect.center().distance(cursor) / max_dist).clamp(0.0, 1.0);
        paint_ghost_rect(&painter, rect, (180.0 - 100.0 * t) as u8);
    }
}

fn calc_primary_ghost_rect(
    page_rect: egui::Rect,
    scale_x: f32,
    scale_y: f32,
    slot: &fotobuch::dto_models::Slot,
    cursor: egui::Pos2,
    cursor_at_drag_start: egui::Pos2,
) -> egui::Rect {
    let w = slot.width_mm as f32 * scale_x;
    let h = slot.height_mm as f32 * scale_y;
    let slot_top_left = egui::pos2(
        page_rect.min.x + slot.x_mm as f32 * scale_x,
        page_rect.min.y + slot.y_mm as f32 * scale_y,
    );
    let grab = cursor_at_drag_start - slot_top_left;
    egui::Rect::from_min_size(cursor - grab, vec2(w, h))
}

fn paint_ghost_rect(painter: &egui::Painter, rect: egui::Rect, alpha: u8) {
    painter.rect_filled(
        rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(100, 149, 237, alpha),
    );
}

fn hit_test_pointer(
    ui: &mut egui::Ui,
    page_rect: egui::Rect,
    layout_page: &fotobuch::dto_models::LayoutPage,
    page_width_mm: f64,
    page_height_mm: f64,
) -> (Option<usize>, bool) {
    match ui.ctx().pointer_hover_pos() {
        None => (None, false),
        Some(pos) => {
            let slot =
                geometry::hit_test_slot(pos, page_rect, layout_page, page_width_mm, page_height_mm);
            (slot, page_rect.contains(pos))
        }
    }
}

fn draw_page_move_highlight(
    ui: &mut egui::Ui,
    state: &GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
    over_page: bool,
) {
    let is_move_drag = !matches!(state.drag, DragState::Idle) && state.drag_mode == DragMode::Move;
    if !is_move_drag || !over_page {
        return;
    }
    let is_src_page =
        matches!(state.drag, DragState::Dragging { src_page, .. } if src_page == page_idx);
    if is_src_page {
        return;
    }
    ui.painter().rect_stroke(
        page_rect,
        0.0,
        egui::Stroke::new(3.0, egui::Color32::from_rgba_unmultiplied(0, 200, 80, 180)),
        egui::StrokeKind::Inside,
    );
}

fn draw_pages(ui: &mut egui::Ui, state: &mut GuiState) {
    let num_pages = state.project_state.layout.len();
    let mut new_hovered: Option<(usize, usize)> = None;
    let mut new_hovered_page: Option<usize> = None;

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
            ui.vertical_centered(|ui| {
                for i in 0..num_pages {
                    let (slot_idx, over_page) = draw_page(ui, state, i);
                    if let Some(slot_idx) = slot_idx
                        && new_hovered.is_none()
                    {
                        new_hovered = Some((i, slot_idx));
                    }
                    if over_page && new_hovered_page.is_none() {
                        new_hovered_page = Some(i);
                    }
                }
            });
        });

    state.hovered_slot = new_hovered;
    state.hovered_page = new_hovered_page;
}
