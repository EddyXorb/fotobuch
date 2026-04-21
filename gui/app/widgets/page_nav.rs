use std::collections::HashSet;

use egui::Align;

use crate::app::pending::PendingCommand;
use crate::state::{GuiState, NavDragState};

pub const NAV_THUMB_MAX_EDGE_PX: u32 = 120;

pub fn draw(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    egui::SidePanel::right("page_nav")
        .resizable(true)
        .min_width(100.0)
        .max_width(200.0)
        .default_width(120.0)
        .show_inside(ui, |ui| show(ui, state, cmds));
}

fn show(ui: &mut egui::Ui, state: &mut GuiState, cmds: &mut HashSet<PendingCommand>) {
    let panel_width = ui.available_width();
    let num_pages = state.project_state.layout.len();

    // Track which page is hovered during nav-drag or central-drag as drop target.
    let mut hovered_nav_page: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for i in 0..num_pages {
                let is_selected = state.scroll_to_page == Some(i);

                // Thumb image or placeholder
                let thumb_size = compute_thumb_size(state, i, panel_width);
                let (response, painter) =
                    ui.allocate_painter(thumb_size, egui::Sense::click_and_drag());
                let rect = response.rect;

                // Draw thumb or grey placeholder
                if let Some(Some(tex)) = state.page_thumb_textures.get(i) {
                    painter.image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(180));
                }

                // Page label below thumb
                ui.label(egui::RichText::new(format!("P{i}")).small());

                // Highlight frame if selected / hovered drop target
                let is_nav_drag_hover =
                    matches!(state.nav_drag, NavDragState::Dragging { .. }) && response.hovered();
                let is_central_drag_hover =
                    !matches!(state.drag, crate::state::DragState::Idle) && response.hovered();

                if is_selected {
                    painter.rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 150, 255)),
                        egui::StrokeKind::Inside,
                    );
                }
                if is_nav_drag_hover || is_central_drag_hover {
                    hovered_nav_page = Some(i);
                    painter.rect_stroke(
                        rect,
                        2.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 200, 80)),
                        egui::StrokeKind::Inside,
                    );
                }

                // Click → scroll central panel to this page
                if response.clicked() {
                    state.scroll_to_page = Some(i);
                    state.selection.clear();
                }

                // --- Nav-Drag: page ↔ page swap (right mouse button) ---
                if response.drag_started_by(egui::PointerButton::Secondary)
                    && matches!(state.nav_drag, NavDragState::Idle)
                {
                    state.nav_drag = NavDragState::Dragging { src_page: i };
                }

                if response.drag_stopped_by(egui::PointerButton::Secondary)
                    && let NavDragState::Dragging { src_page } = state.nav_drag
                {
                    if src_page != i {
                        cmds.insert(PendingCommand::PageSwap {
                            left: src_page,
                            right: i,
                        });
                        if let Some(d) = state.page_dirty.get_mut(src_page) {
                            *d = true;
                        }
                        if let Some(d) = state.page_dirty.get_mut(i) {
                            *d = true;
                        }
                    }
                    state.nav_drag = NavDragState::Idle;
                }

                ui.add_space(4.0);
            }
        });

    // Central-drag over nav thumb → set hovered_page so existing drop logic fires.
    if let Some(nav_page) = hovered_nav_page
        && !matches!(state.drag, crate::state::DragState::Idle)
    {
        state.hovered_page = Some(nav_page);
    }
}

/// Computes thumb display size, preserving page aspect ratio at panel width - margin.
fn compute_thumb_size(state: &GuiState, page_idx: usize, panel_width: f32) -> egui::Vec2 {
    let (pw, ph) = state.project_state.page_dimensions_mm(page_idx);
    let w = (panel_width - 8.0).max(20.0);
    let h = if pw > 0.0 {
        w * (ph as f32 / pw as f32)
    } else {
        w * 1.41 // A4 fallback
    };

    // If a thumb texture is available, use its aspect ratio instead.
    if let Some(Some(tex)) = state.page_thumb_textures.get(page_idx) {
        let sz = tex.size_vec2();
        if sz.x > 0.0 {
            return egui::vec2(w, w * sz.y / sz.x);
        }
    }

    egui::vec2(w, h)
}

/// Applies the scroll-to-page request if this page rect is known.
///
/// Called from `draw_pages` once per page after laying it out.
pub fn apply_scroll_if_needed(
    ui: &mut egui::Ui,
    state: &mut GuiState,
    page_idx: usize,
    page_rect: egui::Rect,
) {
    if state.scroll_to_page == Some(page_idx) {
        ui.scroll_to_rect(page_rect, Some(Align::TOP));
        state.scroll_to_page = None;
    }
}
