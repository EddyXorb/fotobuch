use crate::state::{DataState, HoveredTarget, InteractionState};
use crate::task::BackgroundTask;

use super::super::page_nav;
use super::draw_new_page_slot;
use super::draw_page;

pub(super) fn draw_pages(
    ui: &mut egui::Ui,
    data: &DataState,
    interaction: &mut InteractionState,
    cmds: &mut Vec<BackgroundTask>,
) -> Option<HoveredTarget> {
    let num_pages = data.pages.textures.len();
    let mut hovered: Option<HoveredTarget> = None;

    let rmbactive = ui.input(|i| {
        (i.pointer.secondary_down() || i.pointer.secondary_released()) && !i.pointer.primary_down()
    });

    // Ease-scroll: interpolate toward ease_target each frame.
    if let Some(target_y) = interaction.viewport.scroll.ease_target {
        let current = interaction.viewport.scroll.scroll_y;
        let next = current + (target_y - current) * 0.25;
        interaction.viewport.scroll.pending_scroll_y = Some(next);
        if (target_y - next).abs() < 1.0 {
            interaction.viewport.scroll.ease_target = None;
        } else {
            ui.ctx().request_repaint();
        }
    }

    let pending_scroll = interaction.viewport.scroll.pending_scroll_y.take();
    let mut sa = egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            drag: !rmbactive,
            scroll_bar: true,
            mouse_wheel: true,
        });
    if let Some(y) = pending_scroll {
        sa = sa.vertical_scroll_offset(y);
    }
    let output = sa.show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);

            // Drop zone before page 0 — omitted when a cover is active.
            if !data.project.has_cover() {
                let (_, slot_hovered) = draw_new_page_slot::draw(ui, 0, interaction);
                if hovered.is_none() && slot_hovered {
                    hovered = Some(HoveredTarget::NewPageSlot { at_position: 0 });
                }
            }

            for i in 0..num_pages {
                let (slot_idx, over_page, page_rect, cursor_mm) =
                    draw_page::draw_page(ui, data, interaction, i, cmds);
                if hovered.is_none() {
                    hovered = if let Some(slot) = slot_idx {
                        Some(HoveredTarget::Page {
                            page: i,
                            slot: Some(slot),
                            cursor_mm,
                        })
                    } else if over_page {
                        Some(HoveredTarget::Page {
                            page: i,
                            slot: None,
                            cursor_mm,
                        })
                    } else {
                        None
                    };
                }
                page_nav::apply_scroll_if_needed(ui, interaction, i, page_rect);

                // Drop zone after each page (including after the last page).
                let (_, slot_hovered) = draw_new_page_slot::draw(ui, i + 1, interaction);
                if hovered.is_none() && slot_hovered {
                    hovered = Some(HoveredTarget::NewPageSlot { at_position: i + 1 });
                }
            }

            ui.add_space(60.0);
        });
    });
    interaction.viewport.scroll.scroll_y = output.state.offset.y;
    interaction.viewport.scroll.viewport_top = output.inner_rect.min.y;

    if interaction.viewport.fit_pending && num_pages > 0 {
        let panel_w = output.inner_rect.width();
        if panel_w > 0.0 {
            const MM_TO_PT: f32 = 72.0 / 25.4;
            let max_w_pts = (0..num_pages)
                .map(|i| {
                    let (w_mm, _) = data.project.page_dimensions_mm(i);
                    let (bleed_mm, _) = data.project.page_bleed_margin_mm(i);
                    (w_mm + 2.0 * bleed_mm) as f32 * MM_TO_PT
                })
                .fold(0.0_f32, f32::max);
            if max_w_pts > 0.0 {
                interaction.viewport.zoom = (panel_w / max_w_pts).clamp(0.1, 5.0);
                interaction.viewport.fit_pending = false;
            }
        }
    }

    hovered
}
