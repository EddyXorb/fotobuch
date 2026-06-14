use super::core::rebuild_single_page::rebuild_single_page;
use super::cover::{build_cover_page, split_cover_photos, update_cover_page};
use super::helpers::{build_photo_index, collect_photos_as_groups};
use crate::dto_models::{BookLayoutSolverConfig, LayoutPage, PageMode, PhotoGroup};
use crate::solver::{Request, RequestType, run_solver};
use crate::state_manager::StateManager;
use anyhow::Result;
use std::collections::HashSet;
use tracing::warn;

/// Describes the layout-change strategy for one build or rebuild invocation.
#[derive(Debug, Clone)]
pub enum BuildPlan {
    /// First build or full rebuild: all photos → all pages via the book-layout solver.
    Full,
    /// Incremental build: re-solve only pages whose photos changed since last commit.
    Incremental { pages: Option<Vec<usize>> },
    /// Re-solve a single page with the GA solver.
    Page(usize),
    /// Re-solve a page range with the book-layout solver.
    Range {
        start: usize,
        end: usize,
        flex: usize,
    },
    /// Release build: generate final high-quality PDF, no re-solving.
    Release { force: bool },
}

impl BuildPlan {
    /// Runs the layout solver for this variant and updates `mgr.state.layout`.
    ///
    /// Pure layout step: no cache update, no commit, no PDF.
    /// Returns 0-based indices of pages that were modified.
    pub fn resolve_layout(&self, mgr: &mut StateManager) -> Result<Vec<usize>> {
        match self {
            BuildPlan::Full => resolve_whole_book(mgr),
            BuildPlan::Incremental { pages } => resolve_outdated_pages(mgr, pages.as_deref()),
            BuildPlan::Page(idx) => resolve_single_page(mgr, *idx),
            BuildPlan::Range { start, end, flex } => resolve_range(mgr, *start, *end, *flex),
            BuildPlan::Release { .. } => Ok(Vec::new()),
        }
    }
}

// ── resolve_* handlers ────────────────────────────────────────────────────────

fn resolve_whole_book(mgr: &mut StateManager) -> Result<Vec<usize>> {
    let groups = mgr.state.photos.clone();
    solve_multipage_layout(mgr, &groups, None, None)
}

fn resolve_outdated_pages(
    mgr: &mut StateManager,
    page_filter: Option<&[usize]>,
) -> Result<Vec<usize>> {
    let mut pages = mgr.outdated_pages_indices();
    if let Some(filter) = page_filter {
        pages.retain(|p| filter.contains(p));
    }
    let photo_index = build_photo_index(&mgr.state.photos);
    for &idx in &pages {
        if mgr.state.layout[idx].mode != PageMode::Manual {
            rebuild_single_page(&mut mgr.state, idx, &photo_index)?;
        }
    }
    Ok(pages)
}

fn resolve_single_page(mgr: &mut StateManager, idx: usize) -> Result<Vec<usize>> {
    if idx >= mgr.state.layout.len() {
        anyhow::bail!(
            "Page {} does not exist (layout has {} pages)",
            idx,
            mgr.state.layout.len()
        );
    }
    if mgr.state.layout[idx].mode == PageMode::Manual {
        anyhow::bail!(
            "Cannot rebuild page {}: page is in manual mode. \
             Use `page mode {} a` to switch to auto mode first.",
            idx,
            idx
        );
    }
    let photo_index = build_photo_index(&mgr.state.photos);
    rebuild_single_page(&mut mgr.state, idx, &photo_index)?;
    Ok(vec![idx])
}

fn resolve_range(
    mgr: &mut StateManager,
    start: usize,
    end: usize,
    flex: usize,
) -> Result<Vec<usize>> {
    let effective_start = skip_cover_if_needed(mgr.state.has_cover(), start, end)?;
    let groups = collect_photos_as_groups(&mgr.state, effective_start, end + 1);
    let n = end - effective_start + 1;
    let custom_config = BookLayoutSolverConfig {
        page_min: n.saturating_sub(flex).max(1),
        page_max: n + flex,
        page_target: n,
        ..mgr.state.config.book_layout_solver.clone()
    };
    solve_multipage_layout(
        mgr,
        &groups,
        Some((effective_start, end + 1)),
        Some(custom_config),
    )
}

// ── shared helpers ────────────────────────────────────────────────────────────

/// If cover is active and `start` is 0, skips the cover and returns effective start = 1.
/// Emits a warning in that case. Returns `Err` if the resulting range would be empty.
pub(super) fn skip_cover_if_needed(has_cover: bool, start: usize, end: usize) -> Result<usize> {
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

fn solve_multipage_layout(
    mgr: &mut StateManager,
    groups: &[PhotoGroup],
    range: Option<(usize, usize)>,
    custom_config: Option<BookLayoutSolverConfig>,
) -> Result<Vec<usize>> {
    let solver_config =
        custom_config.unwrap_or_else(|| mgr.state.config.book_layout_solver.clone());
    let ga_config = mgr.state.config.page_layout_solver.clone();
    let book_config = mgr.state.config.book.clone();
    let cover_cfg = &book_config.cover;

    let is_structured_cover = range.is_none() && cover_cfg.active && !cover_cfg.mode.is_free();
    let (cover_files_opt, inner_groups) = if is_structured_cover {
        let n = cover_cfg.mode.required_slots().unwrap();
        let (cover_files, remaining) = split_cover_photos(groups, n);
        (Some(cover_files), remaining)
    } else {
        (None, groups.to_vec())
    };

    let (manual_snapshots, filtered_groups) =
        extract_manual_pages(&mgr.state.layout, &inner_groups, range);

    let mut new_pages = run_solver(&Request {
        request_type: RequestType::MultiPage,
        groups: &filtered_groups,
        config: &solver_config,
        ga_config: &ga_config,
        canvas_config: &book_config,
    })?;

    if let Some(cover_files) = cover_files_opt {
        let inner_count = new_pages.len();
        let cover_page = build_cover_page(cover_cfg, cover_files, inner_count)?;
        new_pages.insert(0, cover_page);
    }

    let range_start = range.map(|(s, _)| s).unwrap_or(0);
    for (orig_abs_idx, manual_page) in manual_snapshots {
        let insert_at = orig_abs_idx
            .saturating_sub(range_start)
            .min(new_pages.len());
        new_pages.insert(insert_at, manual_page);
    }

    let affected = if let Some((start, end)) = range {
        let indices: Vec<usize> = (start..start + new_pages.len()).collect();
        mgr.state.layout.splice(start..end, new_pages);
        indices
    } else {
        let indices: Vec<usize> = (0..new_pages.len()).collect();
        mgr.state.layout = new_pages;
        indices
    };

    if range.is_none_or(|r| r.0 == 0)
        && book_config.cover.active
        && book_config.cover.mode.is_free()
    {
        let photo_index = build_photo_index(&mgr.state.photos);
        update_cover_page(&mut mgr.state, &photo_index)?;
    }

    Ok(affected)
}

/// Extracts manual pages from the layout range and filters their photos from the groups.
///
/// Returns `(snapshots, filtered_groups)` where snapshots are `(original_absolute_index, page)`.
pub(super) fn extract_manual_pages(
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

    // ── skip_cover_if_needed ──────────────────────────────────────────────────

    #[test]
    fn skip_cover_no_cover_returns_start_unchanged() {
        assert_eq!(skip_cover_if_needed(false, 0, 3).unwrap(), 0);
        assert_eq!(skip_cover_if_needed(false, 2, 5).unwrap(), 2);
    }

    #[test]
    fn skip_cover_has_cover_nonzero_start_unchanged() {
        assert_eq!(skip_cover_if_needed(true, 2, 5).unwrap(), 2);
    }

    #[test]
    fn skip_cover_has_cover_start_zero_returns_one() {
        assert_eq!(skip_cover_if_needed(true, 0, 3).unwrap(), 1);
    }

    #[test]
    fn skip_cover_range_zero_zero_errors() {
        assert!(skip_cover_if_needed(true, 0, 0).is_err());
    }

    // ── extract_manual_pages ──────────────────────────────────────────────────

    #[test]
    fn extract_manual_pages_preserves_manual_pages() {
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
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].0, 1);
        assert_eq!(snapshots[0].1.photos, vec!["m1", "m2"]);
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
        let layout = vec![
            make_auto_page(0, &["a"]),
            make_manual_page(1, &["m1"]),
            make_manual_page(2, &["m2"]),
            make_auto_page(3, &["b"]),
        ];
        let groups = vec![make_group("g", &[("m1", 1, 1), ("m2", 1, 1), ("b", 2, 3)])];
        let (snapshots, filtered) = extract_manual_pages(&layout, &groups, Some((2, 4)));
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].0, 2);
        let ids: Vec<_> = filtered
            .iter()
            .flat_map(|g| g.files.iter().map(|f| f.id.as_str()))
            .collect();
        assert!(!ids.contains(&"m2"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"m1"));
    }

    // ── BuildPlan variant construction ────────────────────────────────────────

    #[test]
    fn build_plan_variants_are_constructible() {
        let _ = BuildPlan::Full;
        let _ = BuildPlan::Incremental { pages: None };
        let _ = BuildPlan::Incremental {
            pages: Some(vec![0, 2]),
        };
        let _ = BuildPlan::Page(3);
        let _ = BuildPlan::Range {
            start: 1,
            end: 4,
            flex: 1,
        };
        let _ = BuildPlan::Release { force: false };
        let _ = BuildPlan::Release { force: true };
    }
}
