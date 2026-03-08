//! `fotobuch rebuild` command - Force re-optimization of pages

use crate::cache::preview;
use crate::dto_models::BookLayoutSolverConfig;
use crate::output::typst;
use crate::state_manager::StateManager;
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

use super::build::{
    build_photo_index, collect_photos_as_groups, multipage_build, rebuild_single_page,
    BuildResult, MultiPageParams,
};

/// Scope of rebuild operation
#[derive(Debug, Clone)]
pub enum RebuildScope {
    /// Rebuild all pages (like first build)
    All,
    /// Rebuild single page (forced, even if clean)
    SinglePage(usize),
    /// Rebuild page range with optional flexibility
    Range {
        /// Start page (inclusive)
        start: usize,
        /// End page (inclusive)
        end: usize,
        /// Allow page count to vary by +/- N (default: 0)
        flex: usize,
    },
}

/// Force re-optimization of pages or page ranges
///
/// # Behavior by scope:
///
/// ## Single page: `rebuild 5`
/// - Page-Layout-Solver on page 5, forced even if clean
/// - Photo assignment stays the same, only layout[5].slots is rewritten
/// - Does not trigger Book-Layout-Solver
///
/// ## Page range: `rebuild 3-7`
/// - Book-Layout-Solver on subset: redistribute photos from pages 3-7
/// - Then Page-Layout-Solver for each page in that range
/// - Surrounding pages unchanged
/// - Page count stays the same (5 pages in, 5 pages out) unless --flex is used
///
/// ## With flex: `rebuild 3-7 --flex 2`
/// - Same as range, but solver may use 3-9 pages instead of exactly 5
/// - Useful after `place` when photos are unevenly distributed
///
/// ## All: `rebuild` (no arguments)
/// - Like first build: all photos from photos (top-level), fresh distribution
/// - Book-Layout-Solver + Page-Layout-Solver for all pages
/// - Manual changes in layout are lost (but git-recoverable)
///
/// # Steps
/// 1. StateManager::open() - loads state, commits user edits automatically
/// 2. Preview cache check
/// 3. Run appropriate solver(s)
/// 4. Write fotobuch.yaml
/// 5. Compile Typst -> PDF
/// 6. StateManager::finish() - saves YAML and commits with message
///
/// # Arguments
/// * `project_root` - Path to the project directory
/// * `scope` - Rebuild scope (all, single page, or range)
///
/// # Returns
/// * `BuildResult` with PDF path and statistics
pub fn rebuild(project_root: &Path, scope: RebuildScope) -> Result<BuildResult> {
    let mgr = StateManager::open(project_root)?;

    // Validierung: Layout muss existieren (außer bei All)
    if !matches!(scope, RebuildScope::All) && mgr.state.layout.is_empty() {
        anyhow::bail!(
            "No layout exists. Run `fotobuch build` first, \
             or use `fotobuch rebuild` (without arguments) for a full rebuild."
        );
    }

    // Scope-Validierung
    if let RebuildScope::Range { start, end, .. } = &scope
        && (*start == 0 || *end == 0 || *start > *end || *end > mgr.state.layout.len()) {
            anyhow::bail!(
                "Invalid page range {}-{} (layout has {} pages)",
                start,
                end,
                mgr.state.layout.len()
            );
        }
    if let RebuildScope::SinglePage(n) = &scope
        && (*n == 0 || *n > mgr.state.layout.len()) {
            anyhow::bail!(
                "Invalid page {} (layout has {} pages)",
                n,
                mgr.state.layout.len()
            );
        }

    match scope {
        RebuildScope::SinglePage(n) => rebuild_single(mgr, project_root, n),
        RebuildScope::Range { start, end, flex } => rebuild_range(mgr, project_root, start, end, flex),
        RebuildScope::All => rebuild_all(mgr, project_root),
    }
}

/// Rebuild a single page using the SinglePage solver.
fn rebuild_single(mut mgr: StateManager, project_root: &Path, page: usize) -> Result<BuildResult> {
    // 1. Preview-Cache
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    // 2. Solver — reuse rebuild_single_page from build module
    let photo_index = build_photo_index(&mgr.state.photos);
    rebuild_single_page(&mut mgr.state, page, &photo_index)?;

    // 3. Typst kompilieren
    let pdf_path = typst::compile_preview(project_root, mgr.project_name())?;

    // 4. Fertigstellen — speichert YAML und committed
    mgr.finish(&format!("rebuild: page {}", page))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt: vec![page],
        pages_swapped: vec![],
        images_processed: 0,
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
        nothing_to_do: false,
    })
}

/// Rebuild a page range with optional flexibility.
fn rebuild_range(
    mgr: StateManager,
    project_root: &Path,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<BuildResult> {
    // Collect photos from the range
    let groups = collect_photos_as_groups(&mgr.state, start - 1, end);

    // Build custom config with flex
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
            range: Some((start - 1, end)),
            flex,
            custom_config: Some(custom_config),
            commit_message: format!("rebuild: pages {}-{}", start, end),
            images_processed: 0,
        },
    )
}

/// Rebuild all pages from scratch.
fn rebuild_all(mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
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
        },
    )
}
