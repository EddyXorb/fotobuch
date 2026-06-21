use super::cover_page::{build_cover_page, split_cover_photos, update_cover_page};
use crate::dto_models::{
    BookConfig, BookLayoutSolverConfig, LayoutPage, PageLayoutSolverConfig, PageMode, PhotoFile,
    PhotoGroup, ProjectState, build_photo_index,
};
use crate::solver::{Request, RequestType, run_solver};
use anyhow::Result;
use std::collections::HashSet;

/// Solves a contiguous slice of the book in one GA run.
///
/// `range` is the half-open `[start, end)` layout slice being replaced, or `None`
/// for a full-book solve. Returns the affected 0-based layout indices.
pub(super) fn solve_multipage(
    state: &mut ProjectState,
    groups: &[PhotoGroup],
    range: Option<(usize, usize)>,
    custom_config: Option<BookLayoutSolverConfig>,
) -> Result<Vec<usize>> {
    let mut plan = SolverPlan::build(state, groups, range, custom_config);
    let solved = run_solver(&plan.request())?;
    let pages = plan.assemble(solved)?;
    plan.apply(state, pages)
}

/// Everything the multi-page solver needs, plus the pieces carved out of `groups`
/// *before* solving (a structured cover, manual pages) that have to be merged back
/// into the result *after* solving.
struct SolverPlan {
    groups: Vec<PhotoGroup>,
    solver_config: BookLayoutSolverConfig,
    page_layout_solver_config: PageLayoutSolverConfig,
    book_config: BookConfig,
    /// Structured-cover photos split off the front (full-book solve only).
    cover_files: Option<Vec<PhotoFile>>,
    /// Manual pages held back, keyed by their original absolute layout index.
    manual_snapshots: Vec<(usize, LayoutPage)>,
    range: Option<(usize, usize)>,
}

impl SolverPlan {
    /// Input generation: choose the config, carve out structured cover and manual pages.
    fn build(
        state: &ProjectState,
        groups: &[PhotoGroup],
        range: Option<(usize, usize)>,
        custom_config: Option<BookLayoutSolverConfig>,
    ) -> Self {
        let solver_config =
            custom_config.unwrap_or_else(|| state.config.book_layout_solver.clone());
        let book_config = state.config.book.clone();

        let cover = &book_config.cover;
        let is_structured_cover = range.is_none() && cover.active && !cover.mode.is_free();
        let (cover_files, inner_groups) = if is_structured_cover {
            let n = cover.mode.required_slots().unwrap();
            let (files, remaining) = split_cover_photos(groups, n);
            (Some(files), remaining)
        } else {
            (None, groups.to_vec())
        };

        let (manual_snapshots, groups) = extract_manual_pages(&state.layout, &inner_groups, range);

        Self {
            groups,
            solver_config,
            page_layout_solver_config: state.config.page_layout_solver.clone(),
            book_config,
            cover_files,
            manual_snapshots,
            range,
        }
    }

    /// The solver request, borrowing this plan's owned inputs.
    fn request(&self) -> Request<'_, BookConfig> {
        Request {
            request_type: RequestType::MultiPage {
                config: &self.solver_config,
            },
            groups: &self.groups,
            page_layout_config: &self.page_layout_solver_config,
            canvas_config: &self.book_config,
        }
    }

    /// Merges the carved-out cover and manual pages back into the solver output.
    fn assemble(&mut self, mut pages: Vec<LayoutPage>) -> Result<Vec<LayoutPage>> {
        if let Some(cover_files) = self.cover_files.take() {
            let cover_page = build_cover_page(&self.book_config.cover, cover_files, pages.len())?;
            pages.insert(0, cover_page);
        }

        let range_start = self.range.map_or(0, |(s, _)| s);
        for (orig_abs_idx, manual_page) in std::mem::take(&mut self.manual_snapshots) {
            let insert_at = orig_abs_idx.saturating_sub(range_start).min(pages.len());
            pages.insert(insert_at, manual_page);
        }
        Ok(pages)
    }

    /// Writes the assembled pages into `state.layout`, refreshes a free-mode cover,
    /// and returns the affected 0-based layout indices.
    fn apply(&self, state: &mut ProjectState, pages: Vec<LayoutPage>) -> Result<Vec<usize>> {
        let affected: Vec<usize> = if let Some((start, end)) = self.range {
            let indices = (start..start + pages.len()).collect();
            state.layout.splice(start..end, pages);
            indices
        } else {
            let indices = (0..pages.len()).collect();
            state.layout = pages;
            indices
        };

        let cover = &self.book_config.cover;
        if self.range.is_none_or(|r| r.0 == 0) && cover.active && cover.mode.is_free() {
            let photo_index = build_photo_index(&state.photos);
            update_cover_page(state, &photo_index)?;
        }

        Ok(affected)
    }
}

/// Extracts manual pages from the layout range and filters their photos from the groups.
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
}
