//! `fotobuch rebuild` command - Force re-optimization of pages

use crate::cache::preview;
use crate::commands::CommandOutput;
use crate::dto_models::BookLayoutSolverConfig;
use crate::output::typst;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;

use super::build::{
    BuildResult, MultiPageParams, build_photo_index, collect_photos_as_groups, multipage_build,
    rebuild_single_page,
};
use tracing::warn;

/// Scope of rebuild operation.
///
/// All page references use **0-based array indices** (position in `layout[]`).
/// Cover page (when active) is always at index 0.
#[derive(Debug, Clone)]
pub enum RebuildScope {
    /// Rebuild all pages (like first build)
    All,
    /// Rebuild single page (forced, even if clean).
    /// `page_idx` is a 0-based index into `layout[]` (e.g., `SinglePage(0)` = cover/first page).
    SinglePage(usize),
    /// Rebuild page range with optional flexibility.
    /// `start` and `end` are both 0-based inclusive indices into `layout[]`.
    Range {
        /// Start page index (inclusive, 0-based)
        start: usize,
        /// End page index (inclusive, 0-based)
        end: usize,
        /// Allow page count to vary by +/- N (default: 0)
        flex: usize,
    },
}

/// Force re-optimization of pages or page ranges
///
/// # Behavior by scope:
///
/// ## Single page: `rebuild --page 0`
/// - Slots are recomputed for the given page (GA or deterministic cover solver).
/// - Photo assignment stays the same, only layout[idx].slots is rewritten.
/// - Does not trigger Book-Layout-Solver.
/// - This is the recommended way to refresh the cover after changing cover photos.
///
/// ## Page range: `rebuild --range 3-7`
/// - Book-Layout-Solver on subset, then Page-Layout-Solver for each page.
/// - Surrounding pages unchanged.
/// - Page count stays the same (unless --flex is used).
/// - If range starts at 0 (cover active): cover is skipped; range becomes 1..end
///   (use `rebuild --page 0` to rebuild the cover explicitly).
///
/// ## All: `rebuild` (no arguments)
/// - All inner photos redistributed fresh via Book-Layout-Solver + Page-Layout-Solver.
/// - If cover is active: cover is skipped (use `rebuild --page 0` explicitly).
/// - Manual changes in layout are lost (but git-recoverable).
pub fn rebuild(project_root: &Path, scope: RebuildScope) -> Result<CommandOutput<BuildResult>> {
    let mgr = StateManager::open(project_root)?;

    validate_scope(&scope, &mgr)?;

    match scope {
        RebuildScope::SinglePage(idx) => rebuild_single(mgr, project_root, idx),
        RebuildScope::Range { start, end, flex } => {
            rebuild_range(mgr, project_root, start, end, flex)
        }
        RebuildScope::All => rebuild_all(mgr, project_root),
    }
}

fn validate_scope(scope: &RebuildScope, mgr: &StateManager) -> Result<()> {
    // Layout must exist (except for All)
    if !matches!(scope, RebuildScope::All) && mgr.state.layout.is_empty() {
        anyhow::bail!(
            "No layout exists. Run `fotobuch build` first, \
             or use `fotobuch rebuild` (without arguments) for a full rebuild."
        );
    }

    if let RebuildScope::Range { start, end, .. } = scope
        && (*start > *end || *end >= mgr.state.layout.len())
    {
        anyhow::bail!(
            "Invalid page range {}-{} (layout has {} pages, indices 0..{})",
            start,
            end,
            mgr.state.layout.len(),
            mgr.state.layout.len().saturating_sub(1),
        );
    }
    if let RebuildScope::SinglePage(idx) = scope
        && *idx >= mgr.state.layout.len()
    {
        anyhow::bail!(
            "Invalid page index {} (layout has {} pages, indices 0..{})",
            idx,
            mgr.state.layout.len(),
            mgr.state.layout.len().saturating_sub(1),
        );
    }

    Ok(())
}

/// Rebuild a single page using the SinglePage solver.
fn rebuild_single(
    mut mgr: StateManager,
    project_root: &Path,
    idx: usize,
) -> Result<CommandOutput<BuildResult>> {
    // 1. Check if page is manual - can't rebuild manual pages
    if mgr.state.layout[idx]
        .mode
        .is_some_and(|m| m == crate::dto_models::PageMode::Manual)
    {
        anyhow::bail!(
            "Cannot rebuild page {}: page is in manual mode. Use `page mode {} a` to switch to auto mode first.",
            idx,
            idx
        );
    }

    // 2. Preview-Cache
    let preview_cache_dir = mgr.preview_cache_dir();
    preview::ensure_previews(&mut mgr.state, &preview_cache_dir)?;

    // 3. Solver — reuse rebuild_single_page from build module
    let photo_index = build_photo_index(&mgr.state.photos);
    rebuild_single_page(&mut mgr.state, idx, &photo_index)?;

    // 4. Compile Typst
    let bleed_mm = mgr.state.config.book.bleed_mm;
    let pdf_path = typst::compile_preview(project_root, mgr.project_name(), bleed_mm)?;

    // 5. Save — always commit (even if slots don't change)
    let state = mgr.finish_always(&format!("rebuild: page {}", idx))?;

    Ok(CommandOutput {
        result: BuildResult {
            pdf_path,
            pages_rebuilt: vec![idx],
            pages_swapped: vec![],
            images_processed: 0,
            total_cost: 0.0,
            dpi_warnings: Vec::new(),
            nothing_to_do: false,
        },
        state,
    })
}

/// If cover is active and `start` is 0, skip the cover and return effective start = 1.
/// Emits a warning in that case. Returns `Err` if the resulting range would be empty.
fn skip_cover_if_needed(has_cover: bool, start: usize, end: usize) -> Result<usize> {
    if !has_cover || start != 0 {
        return Ok(start);
    }
    warn!(
        "Cover page (index 0) is excluded from this rebuild. \
         Use `rebuild --page 0` to rebuild it explicitly."
    );
    if end == 0 {
        anyhow::bail!(
            "Range 0-0 contains only the cover page. \
             Use `rebuild --page 0` to rebuild it explicitly."
        );
    }
    Ok(1)
}

/// Rebuild a page range with optional flexibility.
fn rebuild_range(
    mgr: StateManager,
    project_root: &Path,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<CommandOutput<BuildResult>> {
    let effective_start = skip_cover_if_needed(mgr.state.has_cover(), start, end)?;

    let groups = collect_photos_as_groups(&mgr.state, effective_start, end + 1);
    let n = end - effective_start + 1;
    let custom_config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..mgr.state.config.book_layout_solver.clone()
    };

    multipage_build(
        mgr,
        project_root,
        MultiPageParams {
            groups: &groups,
            range: Some((effective_start, end + 1)),
            flex,
            custom_config: Some(custom_config),
            commit_message: format!("rebuild: pages {}-{}", effective_start, end),
            images_processed: 0,
            always_commit: true,
        },
    )
}

/// Rebuild all pages from scratch.
/// Cover page (index 0) is always skipped — use `rebuild --page 0` to rebuild it explicitly.
fn rebuild_all(mgr: StateManager, project_root: &Path) -> Result<CommandOutput<BuildResult>> {
    let layout_len = mgr.state.layout.len();

    let effective_start = if layout_len > 0 {
        skip_cover_if_needed(mgr.state.has_cover(), 0, layout_len - 1)?
    } else {
        0
    };

    let (groups, range) = if effective_start > 0 {
        (
            collect_photos_as_groups(&mgr.state, effective_start, layout_len),
            Some((effective_start, layout_len)),
        )
    } else {
        (mgr.state.photos.clone(), None)
    };

    let photo_count: usize = groups.iter().map(|g| g.files.len()).sum();

    multipage_build(
        mgr,
        project_root,
        MultiPageParams {
            groups: &groups,
            range,
            flex: 0,
            custom_config: None,
            commit_message: format!("rebuild: {} photos redistributed", photo_count),
            images_processed: 0,
            always_commit: true,
        },
    )
}
