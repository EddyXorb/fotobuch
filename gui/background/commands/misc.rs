use fotobuch::commands::build::{BuildConfig, build};
use fotobuch::commands::config::config_set;
use fotobuch::commands::rebuild::{RebuildScope, rebuild};
use fotobuch::commands::undo::{redo, undo};

use crate::background::send_command_done;
use crate::task::BackgroundResult;

pub fn run_undo(rctx: &mut crate::background::RenderCtx<'_>) {
    run_undo_or_redo(undo(rctx.project_root, 1), rctx);
}

pub fn run_redo(rctx: &mut crate::background::RenderCtx<'_>) {
    run_undo_or_redo(redo(rctx.project_root, 1), rctx);
}

fn run_undo_or_redo(
    result: anyhow::Result<fotobuch::commands::CommandOutput<fotobuch::commands::UndoResult>>,
    rctx: &mut crate::background::RenderCtx<'_>,
) {
    match result {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let new_state = output.changed_state;
            let num_pages = new_state.as_ref().map_or(0, |s| s.layout.len());
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(new_state, all_pages, rctx);
        }
    }
}

pub fn run_config_set(key: &str, value: &str, rctx: &mut crate::background::RenderCtx<'_>) {
    match config_set(rctx.project_root, key, value) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(output) => {
            let num_pages = output
                .changed_state
                .as_ref()
                .map(|s| s.layout.len())
                .unwrap_or(0);
            let all_pages: Vec<usize> = (0..num_pages).collect();
            send_command_done(output.changed_state, all_pages, rctx);
        }
    }
}

pub fn run_rebuild_pages(pages: Vec<usize>, rctx: &mut crate::background::RenderCtx<'_>) {
    let mut dirty: Vec<usize> = vec![];
    let mut new_state: Option<fotobuch::dto_models::ProjectState> = None;
    for p in pages {
        match rebuild(rctx.project_root, RebuildScope::SinglePage(p), false) {
            Err(e) => {
                let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(format!(
                    "rebuild page {p}: {e}"
                )));
                return;
            }
            Ok(out) => {
                dirty.push(p);
                if let Some(s) = out.changed_state {
                    new_state = Some(s);
                }
            }
        }
    }
    send_command_done(new_state, dirty, rctx);
}

pub fn run_rebuild_all(rctx: &mut crate::background::RenderCtx<'_>) {
    match rebuild(rctx.project_root, RebuildScope::All, false) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => send_command_done(out.changed_state, out.result.pages_rebuilt, rctx),
    }
}

pub fn run_release_build(rctx: &mut crate::background::RenderCtx<'_>) {
    let cfg = BuildConfig {
        release: true,
        force: false,
        pages: None,
        skip_pdf: false,
    };
    match build(rctx.project_root, &cfg) {
        Err(e) => {
            let _ = rctx
                .result_tx
                .send(BackgroundResult::CommandFailed(e.to_string()));
        }
        Ok(out) => {
            let pdf_path = out.result.pdf_path.clone();
            send_command_done(out.changed_state, out.result.pages_rebuilt, rctx);
            let _ = rctx
                .result_tx
                .send(BackgroundResult::ReleaseDone { pdf_path });
        }
    }
}
