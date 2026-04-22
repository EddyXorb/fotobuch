use crate::state::{GuiState, HoveredTarget};

use super::super::page_nav;
use super::draw_page;

pub(super) fn draw_pages(ui: &mut egui::Ui, state: &mut GuiState) -> Option<HoveredTarget> {
    // Use page_textures.len() rather than layout.len() so that extra pages
    // produced by Typst (e.g. appendix) are also rendered and displayed.
    let num_pages = state.cache.textures.len();
    let mut hovered: Option<HoveredTarget> = None;

    let rmbactive = ui.input(|i| {
        (i.pointer.secondary_down() || i.pointer.secondary_released()) && !i.pointer.primary_down()
    });

    let pending_scroll = state.central_scroll.pending_scroll_y.take();
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
            for i in 0..num_pages {
                let (slot_idx, over_page, page_rect) = draw_page::draw_page(ui, state, i);
                if hovered.is_none() {
                    hovered = if let Some(slot) = slot_idx {
                        Some(HoveredTarget::Page {
                            page: i,
                            slot: Some(slot),
                        })
                    } else if over_page {
                        Some(HoveredTarget::Page {
                            page: i,
                            slot: None,
                        })
                    } else {
                        None
                    };
                }
                page_nav::apply_scroll_if_needed(ui, state, i, page_rect);
            }
        });
    });
    state.central_scroll.scroll_y = output.state.offset.y;
    state.central_scroll.viewport_top = output.inner_rect.min.y;

    hovered
}
