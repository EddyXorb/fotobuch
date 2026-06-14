use super::super::cover::{build_cover_page, split_cover_photos, update_cover_page};
use super::super::helpers::{
    CommitMode, PdfTarget, RenderContext, build_photo_index, render_pdf, update_preview_cache,
};
use super::super::plan::extract_manual_pages;
use crate::commands::CommandOutput;
use crate::commands::build::{BuildOptions, BuildResult};
use crate::dto_models::{BookLayoutSolverConfig, CoverConfig, PhotoFile, PhotoGroup};
use crate::solver::{Request, RequestType, run_solver};
use crate::state_manager::{StateManager, renumber_pages};
use anyhow::Result;
use std::path::Path;

/// Parameters for multipage build/rebuild operations
pub struct MultiPageParams<'a> {
    /// Photo groups to process
    pub groups: &'a [PhotoGroup],
    /// Optional range to replace in existing layout (0-based start, 0-based exclusive end for splice)
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
    pub commit: CommitMode,
    /// Build pipeline side-effect options (PDF generation, cache update)
    pub opts: BuildOptions,
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
) -> Result<CommandOutput<BuildResult>> {
    // 1. Preview-Cache
    let cache_result = if params.opts.skip_cache_update {
        Default::default()
    } else {
        update_preview_cache(&mut mgr)?
    };

    // 2. Determine solver config
    let config = if let Some(ref custom) = params.custom_config {
        custom
    } else {
        &mgr.state.config.book_layout_solver
    };

    // 3. For full rebuilds with a structured cover (non-Free mode): peel off the first N
    //    photos and solve the cover separately so the multipage solver only sees inner pages.
    let cover_cfg = &mgr.state.config.book.cover;
    let (cover_files_opt, inner_groups) = split_cover_for_params(&params, cover_cfg);

    // 3b. Snapshot manual pages so the solver does not redistribute their photos.
    //     Manual pages are restored after the solver run.
    let (manual_snapshots, filtered_groups) =
        extract_manual_pages(&mgr.state.layout, &inner_groups, params.range);

    // 4. Run MultiPage solver (inner pages only when structured cover is active)
    let mut new_pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &filtered_groups,
        config,
        ga_config: &mgr.state.config.page_layout_solver,
        canvas_config: &mgr.state.config.book,
    })?;

    // 5. Build and prepend structured cover page
    if let Some(cover_files) = cover_files_opt {
        let inner_count = new_pages.len();
        let cover_page = build_cover_page(cover_cfg, cover_files, inner_count)?;
        new_pages.insert(0, cover_page);
    }

    // 5b. Splice manual pages back at their original relative positions.
    let range_start = params.range.map(|(s, _)| s).unwrap_or(0);
    for (orig_abs_idx, manual_page) in manual_snapshots {
        let insert_at = orig_abs_idx
            .saturating_sub(range_start)
            .min(new_pages.len());
        new_pages.insert(insert_at, manual_page);
    }

    // 6. Update layout
    let pages_rebuilt = if let Some((start, end)) = params.range {
        // Range rebuild: splice new pages into existing layout
        let pages_rebuilt: Vec<usize> = (start..start + new_pages.len()).collect();
        mgr.state.layout.splice(start..end, new_pages);
        let has_cover = mgr.state.config.book.cover.active;
        renumber_pages(&mut mgr.state.layout, has_cover);
        pages_rebuilt
    } else {
        // Full rebuild: replace entire layout
        let pages_rebuilt: Vec<usize> = (0..new_pages.len()).collect();
        mgr.state.layout = new_pages;
        let has_cover = mgr.state.config.book.cover.active;
        renumber_pages(&mut mgr.state.layout, has_cover);
        pages_rebuilt
    };

    // 7. For Free mode cover: re-solve page 0 with the correct cover spread dimensions
    //    (the MultiPage solver used inner-page canvas dimensions for all pages including
    //    the cover — this step fixes that using the GA solver).
    if params.range.is_none_or(|r| r.0 == 0)
        && mgr.state.config.book.cover.active
        && mgr.state.config.book.cover.mode.is_free()
    {
        let photo_index = build_photo_index(&mgr.state.photos);
        update_cover_page(&mut mgr.state, &photo_index)?;
    }

    let ctx = RenderContext::capture(&mgr);

    // 8. Save and commit
    let changed_state = match params.commit {
        CommitMode::Always => mgr.finish_always(&params.commit_message)?,
        CommitMode::Auto => mgr.finish(&params.commit_message)?,
    };

    // 9. Compile Typst to PDF - do this after commit to ensure yaml is up to date for typst
    let pdf_path = render_pdf(
        project_root,
        &ctx.project_name,
        ctx.bleed_mm,
        PdfTarget::Preview,
        params.opts.skip_pdf,
    )?;

    Ok(CommandOutput {
        result: BuildResult {
            pdf_path,
            pages_rebuilt,
            pages_swapped: vec![],
            images_processed: params.images_processed.max(cache_result.created),
            total_cost: 0.0,
            dpi_warnings: Vec::new(),
            nothing_to_do: false,
        },
        changed_state,
    })
}

fn split_cover_for_params(
    params: &MultiPageParams<'_>,
    cover_cfg: &CoverConfig,
) -> (Option<Vec<PhotoFile>>, Vec<PhotoGroup>) {
    let is_structured_cover =
        params.range.is_none() && cover_cfg.active && !cover_cfg.mode.is_free();

    if is_structured_cover {
        let n = cover_cfg.mode.required_slots().unwrap();
        let (cover_files, remaining) = split_cover_photos(params.groups, n);
        (Some(cover_files), remaining)
    } else {
        (None, params.groups.to_vec())
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::PhotoFile;
    use chrono::Utc;

    fn make_file(id: &str, w: u32, h: u32) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: format!("/photos/{id}.jpg"),
            width_px: w,
            height_px: h,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: "abc".to_string(),
        }
    }

    fn make_group(name: &str, ids: &[(&str, u32, u32)]) -> PhotoGroup {
        PhotoGroup {
            group: name.to_string(),
            sort_key: name.to_string(),
            files: ids.iter().map(|(id, w, h)| make_file(id, *w, *h)).collect(),
        }
    }

    #[test]
    fn split_takes_first_n_from_single_group() {
        let groups = vec![make_group("g1", &[("a", 3, 2), ("b", 4, 3), ("c", 1, 1)])];
        let (cover, remaining) = split_cover_photos(&groups, 1);
        assert_eq!(cover.len(), 1);
        assert_eq!(cover[0].id, "a");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].files.len(), 2);
        assert_eq!(remaining[0].files[0].id, "b");
        assert_eq!(remaining[0].files[1].id, "c");
    }

    #[test]
    fn split_takes_two_across_groups() {
        let groups = vec![
            make_group("g1", &[("a", 3, 2)]),
            make_group("g2", &[("b", 4, 3), ("c", 1, 1)]),
        ];
        let (cover, remaining) = split_cover_photos(&groups, 2);
        assert_eq!(cover.len(), 2);
        assert_eq!(cover[0].id, "a");
        assert_eq!(cover[1].id, "b");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].group, "g2");
        assert_eq!(remaining[0].files[0].id, "c");
    }

    #[test]
    fn split_n_greater_than_total_returns_all_as_cover() {
        let groups = vec![make_group("g1", &[("a", 3, 2)])];
        let (cover, remaining) = split_cover_photos(&groups, 5);
        assert_eq!(cover.len(), 1);
        assert!(remaining.is_empty());
    }

    #[test]
    fn split_empty_groups_returns_empty() {
        let groups: Vec<PhotoGroup> = vec![];
        let (cover, remaining) = split_cover_photos(&groups, 1);
        assert!(cover.is_empty());
        assert!(remaining.is_empty());
    }

    #[test]
    fn split_preserves_group_order_and_sort_key() {
        let groups = vec![
            make_group("a_group", &[("x", 1, 1)]),
            make_group("b_group", &[("y", 1, 1), ("z", 1, 1)]),
        ];
        let (_, remaining) = split_cover_photos(&groups, 1);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].group, "b_group");
        assert_eq!(remaining[0].sort_key, "b_group");
    }
}
