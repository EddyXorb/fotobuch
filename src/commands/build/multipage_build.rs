use super::BuildResult;
use super::helpers::update_preview_pdf;
use crate::cache::preview;
use crate::dto_models::{BookLayoutSolverConfig, PhotoGroup};
use crate::solver::{Request, RequestType, run_solver};
use crate::state_manager::{StateManager, renumber_pages};
use anyhow::Result;
use std::path::Path;

/// Parameters for multipage build/rebuild operations
pub struct MultiPageParams<'a> {
    /// Photo groups to process
    pub groups: &'a [PhotoGroup],
    /// Optional range to replace in existing layout (0-based start, 1-based end for splice)
    /// If None, replaces entire layout
    pub range: Option<(usize, usize)>,
    /// Flexibility in page count (+/- pages)
    pub flex: usize,
    /// Custom book layout solver config (if None, use default from state)
    pub custom_config: Option<BookLayoutSolverConfig>,
    /// Git commit message
    pub commit_message: String,
    /// Number of images processed in cache (for BuildResult)
    pub images_processed: usize,
    /// Whether to always create a commit even if state doesn't change (for rebuild operations)
    pub always_commit: bool,
}

/// Shared multipage build logic used by first_build, rebuild_all, and rebuild_range.
///
/// This function:
/// 1. Ensures preview cache is up to date
/// 2. Runs the MultiPage solver on the given groups
/// 3. Updates the layout (either full replacement or splice)
/// 4. Compiles Typst to PDF
/// 5. Saves and commits
pub fn multipage_build(
    mut mgr: StateManager,
    project_root: &Path,
    params: MultiPageParams,
) -> Result<BuildResult> {
    // 1. Preview-Cache
    let preview_cache_dir = mgr.preview_cache_dir();
    let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir)?;

    // 2. Determine solver config
    let config = if let Some(custom) = params.custom_config {
        custom
    } else {
        mgr.state.config.book_layout_solver.clone()
    };

    // 3. Run MultiPage solver
    let new_pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: params.groups,
        config: &config,
        ga_config: &mgr.state.config.page_layout_solver,
        book_config: &mgr.state.config.book,
    })?;

    // 4. Update layout
    let pages_rebuilt = if let Some((start, end)) = params.range {
        // Range rebuild: splice new pages into existing layout
        let pages_rebuilt: Vec<usize> = (start + 1..=start + new_pages.len()).collect();
        mgr.state.layout.splice(start..end, new_pages);
        let has_cover = mgr.state.config.book.cover.as_ref().is_some_and(|c| c.active);
        renumber_pages(&mut mgr.state.layout, has_cover);
        pages_rebuilt
    } else {
        // Full rebuild: replace entire layout
        let pages_rebuilt: Vec<usize> = (1..=new_pages.len()).collect();
        mgr.state.layout = new_pages;
        pages_rebuilt
    };

    let bleed_mm = mgr.state.config.book.bleed_mm; // need to backup these before mgr gets consumed
    let project_name = mgr.project_name().to_string();

    // 5. Save and commit
    if params.always_commit {
        mgr.finish_always(&params.commit_message)?;
    } else {
        mgr.finish(&params.commit_message)?;
    }

    // 6. Compile Typst to PDF - do this after commit to ensure yaml is up to date for typst
    let pdf_path = update_preview_pdf(project_root, bleed_mm, &project_name)?;

    Ok(BuildResult {
        pdf_path,
        pages_rebuilt,
        pages_swapped: vec![],
        images_processed: params.images_processed.max(cache_result.created),
        total_cost: 0.0,
        dpi_warnings: Vec::new(),
        nothing_to_do: false,
    })
}
