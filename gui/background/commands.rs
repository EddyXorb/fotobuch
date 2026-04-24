use std::path::Path;
use std::path::PathBuf;

use crossbeam::channel::Sender;
use egui::Context;
use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::config::config_set;
use fotobuch::commands::page::{
    DstMove, DstSwap, PageMoveCmd, PagesExpr, SlotExpr, Src, execute_move,
};
use fotobuch::commands::place::{PlaceConfig, place};
use fotobuch::commands::undo::{redo, undo};
use fotobuch::output::typst_world::TypstWorld;

use crate::task::BackgroundResult;

use super::render::send_command_done;

pub(super) fn run_load_photo_thumbnails(
    items: Vec<(String, PathBuf)>,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    repaint_ctx: &Context,
) {
    for (id, source) in items {
        let tx = result_tx.clone();
        let ctx = repaint_ctx.clone();
        pool.spawn(
            move || match crate::thumbnail::load(&source, super::POOL_THUMB_MAX_EDGE_PX) {
                Ok((w, h, pixels)) => {
                    let _ = tx.send(BackgroundResult::PhotoThumbnailReady {
                        id,
                        width: w,
                        height: h,
                        pixels,
                    });
                    ctx.request_repaint();
                }
                Err(e) => {
                    tracing::warn!(%e, photo=%id, "thumb load failed");
                }
            },
        );
    }
}

pub(super) fn run_page_command(
    project_root: &Path,
    cmd: PageMoveCmd,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let move_output = match execute_move(project_root, cmd) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
            return;
        }
        Ok(o) => o,
    };

    if move_output.changed_state.is_none() {
        send_command_done(None, vec![], world, pool, result_tx, ctx, pixel_per_pt);
        return;
    }

    let move_dirty: Vec<usize> = move_output
        .result
        .pages_modified
        .iter()
        .chain(move_output.result.pages_inserted.iter())
        .map(|&p| p as usize)
        .collect();

    let build_cfg = BuildConfig {
        release: false,
        force: false,
        pages: None,
    };
    match build(project_root, &build_cfg) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(build_output) => {
            let new_state = build_output.changed_state.or(move_output.changed_state);
            let mut dirty = move_dirty;
            for p in build_output.result.pages_rebuilt {
                if !dirty.contains(&p) {
                    dirty.push(p);
                }
            }
            send_command_done(new_state, dirty, world, pool, result_tx, ctx, pixel_per_pt);
        }
    }
}

pub(super) fn run_page_swap(
    project_root: &Path,
    left: usize,
    right: usize,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let cmd = PageMoveCmd::Swap {
        left: Src::Pages(PagesExpr::single(left as u32)),
        right: DstSwap::Pages(PagesExpr::single(right as u32)),
    };
    run_page_command(project_root, cmd, world, pool, result_tx, ctx, pixel_per_pt);
}

pub(super) fn run_swap_slots(
    project_root: &Path,
    src_page: usize,
    src_slot: usize,
    dst_page: usize,
    dst_slot: usize,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let cmd = PageMoveCmd::Swap {
        left: Src::Slots {
            page: src_page as u32,
            slots: SlotExpr::single(src_slot as u32),
        },
        right: DstSwap::Slots {
            page: dst_page as u32,
            slots: SlotExpr::single(dst_slot as u32),
        },
    };
    run_page_command(project_root, cmd, world, pool, result_tx, ctx, pixel_per_pt);
}

pub(super) fn run_move_slot(
    project_root: &Path,
    src_page: usize,
    src_slots: Vec<usize>,
    dst_page: usize,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let cmd = PageMoveCmd::Move {
        src: Src::Slots {
            page: src_page as u32,
            slots: SlotExpr::from_list(src_slots.iter().map(|&s| s as u32).collect()),
        },
        dst: DstMove::Page(dst_page as u32),
    };
    run_page_command(project_root, cmd, world, pool, result_tx, ctx, pixel_per_pt);
}

pub(super) fn run_undo(
    project_root: &Path,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    run_undo_or_redo(
        undo(project_root, 1),
        world,
        pool,
        result_tx,
        ctx,
        pixel_per_pt,
    );
}

pub(super) fn run_redo(
    project_root: &Path,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    run_undo_or_redo(
        redo(project_root, 1),
        world,
        pool,
        result_tx,
        ctx,
        pixel_per_pt,
    );
}

fn run_undo_or_redo(
    result: anyhow::Result<fotobuch::commands::CommandOutput<fotobuch::commands::UndoResult>>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    match result {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let new_state = output.changed_state;
            let num_pages = new_state.as_ref().map_or(0, |s| s.layout.len());
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(
                new_state,
                all_pages,
                world,
                pool,
                result_tx,
                ctx,
                pixel_per_pt,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_place_photos(
    project_root: &Path,
    photo_ids: Vec<String>,
    dst_page: Option<usize>,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    let config = PlaceConfig {
        filters: vec![],
        into_page: dst_page,
    };
    match place(project_root, &config) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let dirty = output.result.pages_affected;
            let _ = photo_ids;
            send_command_done(
                output.changed_state,
                dirty,
                world,
                pool,
                result_tx,
                ctx,
                pixel_per_pt,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_config_set(
    project_root: &Path,
    key: &str,
    value: &str,
    world: &mut TypstWorld,
    pool: &rayon::ThreadPool,
    result_tx: &Sender<BackgroundResult>,
    ctx: &Context,
    pixel_per_pt: f32,
) {
    match config_set(project_root, key, value) {
        Err(e) => {
            let _ = result_tx.send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let num_pages = output
                .changed_state
                .as_ref()
                .map(|s| s.layout.len())
                .unwrap_or(0);
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(
                output.changed_state,
                all_pages,
                world,
                pool,
                result_tx,
                ctx,
                pixel_per_pt,
            );
        }
    }
}
