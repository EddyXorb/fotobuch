use super::super::BuildResult;
use super::super::helpers::update_preview_cache;
use super::super::helpers::{build_photo_index, update_preview_pdf};
use super::rebuild_single_page::rebuild_single_page;
use crate::commands::CommandOutput;
use crate::dto_models::{
    BookLayoutSolverConfig, CoverConfig, LayoutPage, PageMode, PhotoFile, PhotoGroup,
};
use crate::solver::cover_solver::{compute_cover_slots, warn_slot_count_mismatch};
use crate::solver::{Request, RequestType, run_solver};
use crate::state_manager::{StateManager, renumber_pages};
use anyhow::Result;
use std::collections::HashSet;
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
    /// Whether to always create a commit even if state doesn't change (for rebuild operations)
    pub always_commit: bool,
    /// Skip PDF generation (Typst compilation )
    pub skip_pdf: bool,
    /// If true, skip updating the preview cache (useful for testing or when caller manages cache separately)
    pub skip_cache_update: bool,
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
    let cache_result = if params.skip_cache_update {
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
    let (cover_files_opt, inner_groups) = split_cover_files(&params, cover_cfg);

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
        rebuild_single_page(&mut mgr.state, 0, &photo_index)?;
    }

    let bleed_mm = mgr.state.config.book.bleed_mm; // need to backup these before mgr gets consumed
    let project_name = mgr.project_name().to_string();

    // 8. Save and commit
    let changed_state = if params.always_commit {
        mgr.finish_always(&params.commit_message)?
    } else {
        mgr.finish(&params.commit_message)?
    };

    // 9. Compile Typst to PDF - do this after commit to ensure yaml is up to date for typst
    let pdf_path = if params.skip_pdf {
        project_root.join(format!("{project_name}.pdf"))
    } else {
        update_preview_pdf(project_root, bleed_mm, &project_name)?
    };

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

fn split_cover_files(
    params: &MultiPageParams<'_>,
    cover_cfg: &CoverConfig,
) -> (Option<Vec<PhotoFile>>, Vec<PhotoGroup>) {
    let is_structured_cover =
        params.range.is_none() && cover_cfg.active && !cover_cfg.mode.is_free();

    let (cover_files_opt, inner_groups) = if is_structured_cover {
        let n = cover_cfg.mode.required_slots().unwrap();
        let (cover_files, remaining) = split_cover_photos(params.groups, n);
        (Some(cover_files), remaining)
    } else {
        (None, params.groups.to_vec())
    };
    (cover_files_opt, inner_groups)
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Splits the first `n` photos (flattened across groups) into cover files, returning
/// the cover files and the rebuilt remaining groups in their original order.
fn split_cover_photos(groups: &[PhotoGroup], n: usize) -> (Vec<PhotoFile>, Vec<PhotoGroup>) {
    let mut flat: Vec<(PhotoFile, &str, &str)> = groups
        .iter()
        .flat_map(|g| {
            g.files
                .iter()
                .map(move |f| (f.clone(), g.group.as_str(), g.sort_key.as_str()))
        })
        .collect();

    let cover_files: Vec<PhotoFile> = flat.drain(..n.min(flat.len())).map(|(f, _, _)| f).collect();

    // Reconstruct remaining groups preserving original order and group names
    let mut remaining: Vec<PhotoGroup> = Vec::new();
    for (file, group_name, sort_key) in flat {
        if let Some(g) = remaining.iter_mut().find(|g| g.group == group_name) {
            g.files.push(file);
        } else {
            remaining.push(PhotoGroup {
                group: group_name.to_string(),
                sort_key: sort_key.to_string(),
                files: vec![file],
            });
        }
    }

    (cover_files, remaining)
}

/// Creates a cover `LayoutPage` (index 0) from the given files using the deterministic
/// cover solver. `inner_page_count` is needed for spine width calculation.
fn build_cover_page(
    cover: &CoverConfig,
    files: Vec<PhotoFile>,
    inner_page_count: usize,
) -> Result<LayoutPage> {
    let ratios: Vec<f64> = files.iter().map(|f| f.aspect_ratio()).collect();
    warn_slot_count_mismatch(cover.mode, files.len());
    let slots = compute_cover_slots(cover, &ratios, inner_page_count)?;
    Ok(LayoutPage {
        page: 0,
        photos: files.into_iter().map(|f| f.id).collect(),
        slots,
        mode: PageMode::Auto,
    })
}

/// Extracts manual pages from the layout range, returning a sorted snapshot and
/// filtered groups that exclude those pages' photos.
///
/// Returns `(snapshots, filtered_groups)` where snapshots are `(original_absolute_index, page)`.
fn extract_manual_pages(
    layout: &[LayoutPage],
    groups: &[PhotoGroup],
    range: Option<(usize, usize)>,
) -> (Vec<(usize, LayoutPage)>, Vec<PhotoGroup>) {
    let (range_start, range_end) = match range {
        Some((s, e)) => (s, e),
        None => (0, layout.len()),
    };

    let snapshots: Vec<(usize, LayoutPage)> = layout[range_start..range_end.min(layout.len())]
        .iter()
        .enumerate()
        .filter(|(_, p)| p.mode == PageMode::Manual)
        .map(|(i, p)| (range_start + i, p.clone()))
        .collect();

    if snapshots.is_empty() {
        return (snapshots, groups.to_vec());
    }

    let manual_ids: HashSet<&str> = snapshots
        .iter()
        .flat_map(|(_, p)| p.photos.iter().map(String::as_str))
        .collect();

    let filtered: Vec<PhotoGroup> = groups
        .iter()
        .filter_map(|g| {
            let files: Vec<_> = g
                .files
                .iter()
                .filter(|f| !manual_ids.contains(f.id.as_str()))
                .cloned()
                .collect();
            if files.is_empty() {
                None
            } else {
                Some(PhotoGroup {
                    group: g.group.clone(),
                    sort_key: g.sort_key.clone(),
                    files,
                })
            }
        })
        .collect();

    (snapshots, filtered)
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

    fn make_auto_page(idx: usize, ids: &[&str]) -> LayoutPage {
        LayoutPage {
            page: idx,
            photos: ids.iter().map(|s| s.to_string()).collect(),
            slots: vec![],
            mode: PageMode::Auto,
        }
    }

    fn make_manual_page(idx: usize, ids: &[&str]) -> LayoutPage {
        LayoutPage {
            page: idx,
            photos: ids.iter().map(|s| s.to_string()).collect(),
            slots: vec![],
            mode: PageMode::Manual,
        }
    }

    #[test]
    fn book_layout_solver_preserves_manual_pages() {
        // layout: [auto@0, manual@1, auto@2]
        let layout = vec![
            make_auto_page(0, &["a", "b"]),
            make_manual_page(1, &["m1", "m2"]),
            make_auto_page(2, &["c"]),
        ];
        let groups = vec![
            make_group("g1", &[("a", 3, 2), ("b", 4, 3)]),
            make_group("g_manual", &[("m1", 1, 1), ("m2", 1, 1)]),
            make_group("g2", &[("c", 2, 3)]),
        ];

        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, None);
        // manual page at index 1 should be snapshotted
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].0, 1); // original absolute index
        assert_eq!(snapshots[0].1.photos, vec!["m1", "m2"]);
        // filtered groups should not contain m1/m2
        let all_ids: Vec<_> = filtered
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.as_str()))
            .collect();
        assert!(!all_ids.contains(&"m1"));
        assert!(!all_ids.contains(&"m2"));
        assert!(all_ids.contains(&"a"));
        assert!(all_ids.contains(&"c"));
    }

    #[test]
    fn extract_manual_pages_no_manual_returns_unchanged() {
        let layout = vec![make_auto_page(0, &["a"]), make_auto_page(1, &["b"])];
        let groups = vec![make_group("g", &[("a", 1, 1), ("b", 1, 1)])];
        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, None);
        assert!(snapshots.is_empty());
        assert_eq!(filtered.len(), groups.len());
    }

    #[test]
    fn extract_manual_pages_range_only_within_range() {
        // layout: [auto@0, manual@1, manual@2, auto@3]
        let layout = vec![
            make_auto_page(0, &["a"]),
            make_manual_page(1, &["m1"]),
            make_manual_page(2, &["m2"]),
            make_auto_page(3, &["b"]),
        ];
        let groups = vec![make_group("g", &[("m1", 1, 1), ("m2", 1, 1), ("b", 2, 3)])];
        // range covers only indices 2..4 (m2 and b)
        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, Some((2, 4)));
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].0, 2);
        let ids: Vec<_> = filtered
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.as_str()))
            .collect();
        assert!(!ids.contains(&"m2"));
        assert!(ids.contains(&"b"));
        // m1 is outside range, should still be in filtered (not a manual page for this range)
        assert!(ids.contains(&"m1"));
    }
}
