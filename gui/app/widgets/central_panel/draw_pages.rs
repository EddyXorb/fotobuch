use crate::state::GuiState;

use super::super::page_nav;
use super::draw_page;

pub(super) fn draw_pages(ui: &mut egui::Ui, state: &mut GuiState) {
    // Use page_textures.len() rather than layout.len() so that extra pages
    // produced by Typst (e.g. appendix) are also rendered and displayed.
    let num_pages = state.page_textures.len();
    let mut new_hovered: Option<(usize, usize)> = None;
    let mut new_hovered_page: Option<usize> = None;

    let rmbactive = ui.input(|i| {
        (i.pointer.secondary_down() || i.pointer.secondary_released()) && !i.pointer.primary_down()
    });

    let pending_scroll = state.pending_central_scroll_y.take();
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
                if let Some(slot_idx) = slot_idx
                    && new_hovered.is_none()
                {
                    new_hovered = Some((i, slot_idx));
                }
                if over_page && new_hovered_page.is_none() {
                    new_hovered_page = Some(i);
                }
                page_nav::apply_scroll_if_needed(ui, state, i, page_rect);
            }
        });
    });
    state.central_scroll_y = output.state.offset.y;
    state.central_viewport_top = output.inner_rect.min.y;

    state.hovered_slot = new_hovered;
    state.hovered_page = new_hovered_page;
}
