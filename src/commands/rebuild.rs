//! `fotobuch rebuild` command - Force re-optimization of pages

use crate::cache::preview;
use crate::dto_models::{BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup, ProjectState};
use crate::output::typst;
use crate::solver::{run_solver, Request, RequestType};
use crate::state_manager::StateManager;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicUsize;

use super::build::{rebuild_single_page, BuildResult};

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
    let typ_path = format!("{}.typ", mgr.project_name());
    let pdf_path = typst::compile_preview(project_root, &typ_path)?;

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
    mut mgr: StateManager,
    project_root: &Path,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<BuildResult> {
    // 1. Preview-Cache
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    // 2. Fotos aus Bereich als PhotoGroups rekonstruieren
    let groups = collect_photos_as_groups(&mgr.state, start - 1, end);

    // 3. Solver mit angepassten Seitengrenzen
    let n = end - start + 1;
    let config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..mgr.state.config.book_layout_solver.clone()
    };

    let new_pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &groups,
        config: &config,
        ga_config: &mgr.state.config.page_layout_solver,
        book_config: &mgr.state.config.book,
    })?;

    let pages_rebuilt: Vec<usize> = (start..start + new_pages.len()).collect();

    // 4. Layout aktualisieren + renumbern
    mgr.state.layout.splice((start - 1)..end, new_pages);
    renumber_pages(&mut mgr.state.layout);

    // 5. Typst kompilieren
    let typ_path = format!("{}.typ", mgr.project_name());
    let pdf_path = typst::compile_preview(project_root, &typ_path)?;

    // 6. Fertigstellen — speichert YAML und committed
    mgr.finish(&format!("rebuild: pages {}-{}", start, end))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        pages_swapped: vec![],
        images_processed: 0,
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
        nothing_to_do: false,
    })
}

/// Rebuild all pages from scratch.
fn rebuild_all(mut mgr: StateManager, project_root: &Path) -> Result<BuildResult> {
    // 1. Preview-Cache
    let progress = AtomicUsize::new(0);
    let preview_cache_dir = mgr.preview_cache_dir();
    preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

    // 2. Solver MultiPage auf alle Photos (inkl. unplaced)
    let pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &mgr.state.photos,
        config: &mgr.state.config.book_layout_solver,
        ga_config: &mgr.state.config.page_layout_solver,
        book_config: &mgr.state.config.book,
    })?;

    let pages_rebuilt: Vec<usize> = (1..=pages.len()).collect();
    let page_count = pages.len();
    mgr.state.layout = pages;

    // 3. Typst kompilieren
    let typ_path = format!("{}.typ", mgr.project_name());
    let pdf_path = typst::compile_preview(project_root, &typ_path)?;

    // 4. Fertigstellen — speichert YAML und committed
    mgr.finish(&format!("rebuild: {} pages", page_count))?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        pages_swapped: vec![],
        images_processed: 0,
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
        nothing_to_do: false,
    })
}

/// Sammelt alle Fotos aus dem Seitenbereich und rekonstruiert PhotoGroups.
///
/// start: 0-basiert (inclusive)
/// end: 1-basiert (= exklusiv, passt zu layout[start..end] und splice)
fn collect_photos_as_groups(state: &ProjectState, start: usize, end: usize) -> Vec<PhotoGroup> {
    let photo_index = build_photo_index(&state.photos);

    // Photo-IDs aus dem Bereich sammeln
    let page_photo_ids: Vec<&str> = state.layout[start..end]
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    // Nach Originalgruppe aufteilen
    let mut groups_map: HashMap<&str, Vec<PhotoFile>> = HashMap::new();
    for id in &page_photo_ids {
        if let Some((pf, group_name)) = photo_index.get(*id) {
            groups_map
                .entry(group_name)
                .or_default()
                .push((*pf).clone());
        }
    }

    // sort_key aus state.photos übernehmen
    let group_sort_keys: HashMap<&str, &str> = state
        .photos
        .iter()
        .map(|g| (g.group.as_str(), g.sort_key.as_str()))
        .collect();

    let mut groups: Vec<PhotoGroup> = groups_map
        .into_iter()
        .map(|(name, files)| PhotoGroup {
            group: name.to_string(),
            sort_key: group_sort_keys.get(name).unwrap_or(&"").to_string(),
            files,
        })
        .collect();

    groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    groups
}

/// Nummeriert alle LayoutPage.page Felder sequenziell (1-basiert).
fn renumber_pages(layout: &mut [LayoutPage]) {
    for (i, page) in layout.iter_mut().enumerate() {
        page.page = i + 1;
    }
}

/// Maps photo ID to (PhotoFile, group_name).
fn build_photo_index(photos: &[PhotoGroup]) -> HashMap<String, (PhotoFile, String)> {
    photos
        .iter()
        .flat_map(|group| {
            group
                .files
                .iter()
                .map(move |file| (file.id.clone(), (file.clone(), group.group.clone())))
        })
        .collect()
}
