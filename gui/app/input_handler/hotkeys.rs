use crate::task::BackgroundTask;

use crate::app::rebuild::{PagesForRebuild, selected_pages_for_rebuild};
use crate::state::{self, DataState, HoveredTarget, InteractionState, SlotSelection, WeightSlider};
use fotobuch::commands::PlaceDst;

pub(super) fn handle_drag_mode_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::M)) {
        interaction.drag.mode = interaction.drag.mode.toggle();
    }
}

pub(super) fn handle_timings_toggle(data: &mut DataState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
        data.timings.show = !data.timings.show;
    }
}

pub(super) fn handle_zoom(interaction: &mut InteractionState, ctx: &egui::Context) {
    let delta = ctx.input(|i| {
        if i.modifiers.ctrl {
            i.zoom_delta()
        } else {
            1.0
        }
    });
    if delta == 1.0 {
        return;
    }
    let old_zoom = interaction.viewport.zoom;
    interaction.viewport.zoom = state::apply_zoom_delta(old_zoom, delta);
    let ratio = interaction.viewport.zoom / old_zoom;
    let cursor_y = ctx
        .pointer_hover_pos()
        .map_or(interaction.viewport.scroll.viewport_top, |p| p.y);
    let rel = cursor_y - interaction.viewport.scroll.viewport_top;
    interaction.viewport.scroll.pending_scroll_y =
        Some(interaction.viewport.scroll.scroll_y * ratio + rel * (ratio - 1.0));
}

pub(super) fn handle_undo_redo(ctx: &egui::Context, cmds: &mut Vec<BackgroundTask>) {
    let redo = ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
            || i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
    });
    if redo {
        cmds.push(BackgroundTask::Redo);
        return;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
        cmds.push(BackgroundTask::Undo);
    }
}

pub(super) fn handle_escape(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        interaction.drag.active = crate::state::ActiveDrag::Idle;
        interaction.selections.slots.clear();
        interaction.selections.nav_pages.clear();
        interaction.context_menu = None;
        interaction.weight_slider = WeightSlider::Closed;
    }
}

pub(super) fn handle_select_all(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
        return;
    }
    if matches!(interaction.hovered, Some(HoveredTarget::PoolItem(_))) {
        let all_ids = data
            .project
            .photos
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.clone()));
        interaction.selections.photos.select_all(all_ids);
        return;
    }
    let current_page = interaction
        .hovered
        .as_ref()
        .and_then(|h| h.slot())
        .map(|(p, _)| p)
        .or(interaction.selections.slots.page);
    if let Some(page) = current_page {
        let slot_count = data
            .project
            .layout
            .get(page)
            .map(|lp| lp.slots.len())
            .unwrap_or(0);
        interaction.selections.slots.select_all_on(page, slot_count);
    }
}

pub(super) fn handle_config_panel_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Comma)) {
        interaction.config.open = !interaction.config.open;
    }
}

pub(super) fn handle_place_hotkey(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P)) {
        return;
    }
    let ids = interaction.selections.photos.ids();
    let photo_ids = if ids.is_empty() {
        data.project
            .photos
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.clone()))
            .collect::<Vec<_>>()
    } else {
        ids
    };
    if photo_ids.is_empty() {
        return;
    }
    cmds.push(BackgroundTask::Place {
        photo_ids,
        dst: match interaction
            .hovered
            .as_ref()
            .and_then(HoveredTarget::central_page)
        {
            Some(p) => PlaceDst::Page(p),
            None => PlaceDst::Auto,
        },
    });
}

pub(super) fn handle_delete(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
        return;
    }

    // 1) Slot-Selektion gewinnt (Phase 5).
    if let Some(page) = interaction.selections.slots.page
        && !interaction.selections.slots.is_empty()
    {
        cmds.push(BackgroundTask::Unplace {
            page,
            slots: interaction.selections.slots.slots_on_active_page(),
        });
        return;
    }

    // 2) Pool-Selektion.
    let pool_ids = interaction.selections.photos.ids();
    if !pool_ids.is_empty() {
        cmds.push(BackgroundTask::RemovePhotos {
            photo_ids: pool_ids,
        });
        return;
    }

    // 3) Nav-Selektion — Cover wird gefiltert.
    let nav_sel = interaction.selections.nav_pages.items();
    if !nav_sel.is_empty() {
        let pages: Vec<usize> = if data.project.has_cover() {
            nav_sel.into_iter().filter(|&p| p != 0).collect()
        } else {
            nav_sel
        };
        if !pages.is_empty() {
            cmds.push(BackgroundTask::DeletePages { pages });
        }
    }
}

pub(super) fn handle_rebuild(
    _data: &DataState,
    interaction: &InteractionState,
    ctx: &egui::Context,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
        return;
    }
    if let PagesForRebuild::Selected(pages) = selected_pages_for_rebuild(interaction) {
        cmds.push(BackgroundTask::RebuildPages { pages });
    }
}

pub(super) fn handle_goto_toggle(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::G)) {
        interaction.goto_open = !interaction.goto_open;
    }
}

pub(super) fn handle_home_end(
    interaction: &mut InteractionState,
    data: &DataState,
    ctx: &egui::Context,
) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
        interaction.viewport.scroll_to_page = Some(0);
    } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::End)) {
        let last = data.project.layout.len().saturating_sub(1);
        interaction.viewport.scroll_to_page = Some(last);
    }
}

pub(super) fn handle_fit_width(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Num0)) {
        interaction.viewport.fit_pending = true;
    }
}

pub(super) fn handle_release_build(ctx: &egui::Context, cmds: &mut Vec<BackgroundTask>) {
    if ctx
        .input_mut(|i| i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::B))
    {
        cmds.push(BackgroundTask::ReleaseBuild);
    }
}

pub(super) fn handle_add_hotkey(interaction: &mut InteractionState, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
        interaction.add_dialog.open = true;
    }
}

pub(super) fn handle_mode_toggle(
    data: &DataState,
    interaction: &InteractionState,
    ctx: &egui::Context,
    cmds: &mut Vec<BackgroundTask>,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::A)) {
        return;
    }
    let page = interaction
        .selections
        .slots
        .page
        .or_else(|| interaction.hovered.as_ref().and_then(|h| h.page_idx()));
    let Some(page) = page else { return };
    let Some(lp) = data.project.layout.get(page) else {
        return;
    };
    use fotobuch::dto_models::PageMode;
    let new_mode = match lp.mode {
        PageMode::Auto => PageMode::Manual,
        PageMode::Manual => PageMode::Auto,
    };
    cmds.push(BackgroundTask::SetPageMode {
        page,
        mode: new_mode,
    });
}

pub(super) fn handle_click(interaction: &mut InteractionState, ctx: &egui::Context) {
    if !ctx.input(|i| i.pointer.primary_clicked()) {
        return;
    }
    let modifiers = ctx.input(|i| i.modifiers);
    if let Some((page, slot)) = interaction.hovered.as_ref().and_then(|h| h.slot()) {
        interaction.selections.nav_pages.clear();
        if modifiers.shift {
            interaction.selections.slots.range_to(page, slot);
        } else if modifiers.ctrl || modifiers.command {
            interaction.selections.slots.toggle(page, slot);
        } else {
            interaction.selections.slots = SlotSelection::single(page, slot);
        }
    } else {
        interaction.selections.slots.clear();
    }
}

pub(super) fn handle_weight_hotkey(
    data: &DataState,
    interaction: &mut InteractionState,
    ctx: &egui::Context,
) {
    if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::W)) {
        return;
    }
    // Toggle: close if already open.
    if interaction.weight_slider.is_open() {
        interaction.weight_slider = WeightSlider::Closed;
        return;
    }
    let Some(page) = interaction.selections.slots.page else {
        return;
    };
    let slots = interaction.selections.slots.slots_on_active_page();
    if slots.is_empty() {
        return;
    }
    let initial = compute_initial_weight(data, page, &slots);
    let pos = ctx
        .pointer_hover_pos()
        .unwrap_or_else(|| egui::pos2(200.0, 200.0));
    interaction.weight_slider = WeightSlider::Open {
        page,
        slots,
        screen_pos: pos,
        value: initial,
    };
}

fn compute_initial_weight(data: &DataState, page: usize, slots: &[usize]) -> f64 {
    let lp = match data.project.layout.get(page) {
        Some(lp) => lp,
        None => return 1.0,
    };
    let weights: Vec<f64> = slots
        .iter()
        .filter_map(|&s| {
            let photo_id = lp.photos.get(s)?;
            data.project
                .photos
                .iter()
                .flat_map(|g| g.files.iter())
                .find(|f| &f.id == photo_id)
                .map(|f| f.area_weight)
        })
        .collect();
    if weights.is_empty() {
        return 1.0;
    }
    let avg = weights.iter().sum::<f64>() / weights.len() as f64;
    // Round to nearest 0.1.
    (avg * 10.0).round() / 10.0
}
