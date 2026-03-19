//! `fotobuch rebuild` command - Force re-optimization of pages

use crate::cache::preview;
use crate::dto_models::BookLayoutSolverConfig;
use crate::output::typst;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;

use super::build::{
    BuildResult, MultiPageParams, build_photo_index, collect_photos_as_groups, multipage_build,
    rebuild_single_page,
};

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
/// - Page-Layout-Solver on the given page, forced even if clean
/// - Photo assignment stays the same, only layout[idx].slots is rewritten
/// - Does not trigger Book-Layout-Solver
///
/// ## Page range: `rebuild --range 3-7`
/// - If range includes cover (index 0, active): cover is solved first with SinglePage solver,
///   remaining pages in range with Book-Layout-Solver + Page-Layout-Solver
/// - Otherwise: Book-Layout-Solver on subset, then Page-Layout-Solver for each page
/// - Surrounding pages unchanged
/// - Page count stays the same (unless --flex is used)
///
/// ## All: `rebuild` (no arguments)
/// - If cover is active: cover solved with SinglePage, all inner pages redistributed fresh
/// - Otherwise: all photos from photos (top-level), fresh distribution
/// - Book-Layout-Solver + Page-Layout-Solver for all inner pages
/// - Manual changes in layout are lost (but git-recoverable)
pub fn rebuild(project_root: &Path, scope: RebuildScope) -> Result<BuildResult> {
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
fn rebuild_single(mut mgr: StateManager, project_root: &Path, idx: usize) -> Result<BuildResult> {
    // 1. Preview-Cache
    let preview_cache_dir = mgr.preview_cache_dir();
    preview::ensure_previews(&mgr.state, &preview_cache_dir)?;

    // 2. Solver — reuse rebuild_single_page from build module
    let photo_index = build_photo_index(&mgr.state.photos);
    rebuild_single_page(&mut mgr.state, idx, &photo_index)?;

    // 3. Compile Typst
    let bleed_mm = mgr.state.config.book.bleed_mm;
    let pdf_path = typst::compile_preview(project_root, mgr.project_name(), bleed_mm)?;

    // 4. Save — always commit (even if slots don't change)
    mgr.finish_always(&format!("rebuild: page {}", idx))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: vec![idx],
        pages_swapped: vec![],
        images_processed: 0,
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
        nothing_to_do: false,
    })
}

/// Rebuild a page range with optional flexibility.
/// If the range includes the cover page (index 0, active), it is solved first with
/// SinglePage solver; the rest of the range uses MultiPage solver.
fn rebuild_range(
    mut mgr: StateManager,
    project_root: &Path,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<BuildResult> {
    let has_cover = mgr.state.config.book.cover.as_ref().is_some_and(|c| c.active);
    let range_includes_cover = has_cover && start == 0;

    if range_includes_cover {
        // 1. Rebuild cover (index 0) with SinglePage solver
        let preview_cache_dir = mgr.preview_cache_dir();
        preview::ensure_previews(&mgr.state, &preview_cache_dir)?;
        let photo_index = build_photo_index(&mgr.state.photos);
        rebuild_single_page(&mut mgr.state, 0, &photo_index)?;

        // 2. If range is only the cover, we're done
        if end == 0 {
            let bleed_mm = mgr.state.config.book.bleed_mm;
            let pdf_path = typst::compile_preview(project_root, mgr.project_name(), bleed_mm)?;
            mgr.finish_always("rebuild: page 0 (cover)")?;
            return Ok(BuildResult {
                pdf_path,
                pages_rebuilt: vec![0],
                pages_swapped: vec![],
                images_processed: 0,
                total_cost: 0.0,
                dpi_warnings: Vec::new(),
                nothing_to_do: false,
            });
        }

        // 3. Rebuild remaining range (1..=end) with MultiPage
        let inner_start = 1usize;
        let inner_end = end;
        let groups = collect_photos_as_groups(&mgr.state, inner_start, inner_end + 1);
        let n = inner_end - inner_start + 1;
        let custom_config = BookLayoutSolverConfig {
            page_min: n.saturating_sub(flex).max(1),
            page_max: n + flex,
            page_target: n,
            ..mgr.state.config.book_layout_solver.clone()
        };

        let mut result = multipage_build(
            mgr,
            project_root,
            MultiPageParams {
                groups: &groups,
                range: Some((inner_start, inner_end + 1)),
                flex,
                custom_config: Some(custom_config),
                commit_message: format!("rebuild: pages {}-{} (cover via singlesolver)", start, end),
                images_processed: 0,
                always_commit: true,
            },
        )?;

        // Prepend cover (index 0) to pages_rebuilt
        result.pages_rebuilt.insert(0, 0);
        Ok(result)
    } else {
        // No cover in range — standard MultiPage rebuild
        let groups = collect_photos_as_groups(&mgr.state, start, end + 1);
        let n = end - start + 1;
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
                range: Some((start, end + 1)),
                flex,
                custom_config: Some(custom_config),
                commit_message: format!("rebuild: pages {}-{}", start, end),
                images_processed: 0,
                always_commit: true,
            },
        )
    }
}

/// Rebuild all pages from scratch.
/// If cover is active: cover is solved with SinglePage, inner pages redistributed fresh.
fn rebuild_all(mut mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
    let has_cover = mgr.state.config.book.cover.as_ref().is_some_and(|c| c.active);
    let cover_exists = has_cover && !mgr.state.layout.is_empty();

    if cover_exists {
        // 1. Rebuild cover (index 0) with SinglePage solver
        let preview_cache_dir = mgr.preview_cache_dir();
        preview::ensure_previews(&mgr.state, &preview_cache_dir)?;
        let photo_index = build_photo_index(&mgr.state.photos);
        rebuild_single_page(&mut mgr.state, 0, &photo_index)?;

        // 2. Rebuild all inner pages (index 1..) with MultiPage
        //    Collect all photos that are NOT on the cover
        let cover_photos: std::collections::HashSet<String> =
            mgr.state.layout[0].photos.iter().cloned().collect();

        let inner_groups: Vec<_> = mgr
            .state
            .photos
            .iter()
            .filter_map(|g| {
                let files: Vec<_> = g
                    .files
                    .iter()
                    .filter(|f| !cover_photos.contains(&f.id))
                    .cloned()
                    .collect();
                if files.is_empty() {
                    None
                } else {
                    Some(crate::dto_models::PhotoGroup {
                        group: g.group.clone(),
                        sort_key: g.sort_key.clone(),
                        files,
                    })
                }
            })
            .collect();

        let inner_page_count: usize = inner_groups.iter().map(|g| g.files.len()).sum();
        let layout_len = mgr.state.layout.len(); // capture before mgr is consumed

        let mut result = multipage_build(
            mgr,
            project_root,
            MultiPageParams {
                groups: &inner_groups,
                range: Some((1, layout_len)), // replace layout[1..layout_len]
                flex: 0,
                custom_config: None,
                commit_message: format!(
                    "rebuild: {} inner photos redistributed (cover via singlesolver)",
                    inner_page_count
                ),
                images_processed: 0,
                always_commit: true,
            },
        )?;

        // Prepend cover index
        result.pages_rebuilt.insert(0, 0);
        Ok(result)
    } else {
        // No cover — standard full rebuild
        let groups = mgr.state.photos.clone();
        let page_count = groups.iter().map(|g| g.files.len()).sum::<usize>();

        multipage_build(
            mgr,
            project_root,
            MultiPageParams {
                groups: &groups,
                range: None,
                flex: 0,
                custom_config: None,
                commit_message: format!("rebuild: {} photos redistributed", page_count),
                images_processed: 0,
                always_commit: true,
            },
        )
    }
}
