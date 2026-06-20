use crate::dto_models::{CanvasConfig, CoverConfig, LayoutPage, PageMode, PhotoFile, PhotoGroup};
use crate::run_solver;
use crate::solver::cover_solver::{compute_cover_slots, warn_slot_count_mismatch};
use crate::solver::{Request, RequestType};
use anyhow::Result;
use std::collections::HashMap;

/// Creates a new cover `LayoutPage` using the deterministic cover solver (structured mode only).
pub(super) fn build_cover_page(
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

/// Updates the existing cover page (index 0) in `state.layout`.
/// Dispatches to the GA solver (free mode) or the deterministic cover solver (structured mode).
pub(super) fn update_cover_page(
    state: &mut crate::dto_models::ProjectState,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    let files: Vec<PhotoFile> = state.layout[0]
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id).map(|(f, _)| f.clone()))
        .collect();

    if state.config.book.cover.mode.is_free() {
        update_cover_free(state, files)
    } else {
        update_cover_structured(state, files, photo_index)
    }
}

fn update_cover_free(
    state: &mut crate::dto_models::ProjectState,
    files: Vec<PhotoFile>,
) -> Result<()> {
    let cover = &state.config.book.cover;
    let inner_page_count = state.layout.len() - 1;
    let spread_config = CoverCanvasConfig {
        cover,
        inner_page_count,
    };
    let group = PhotoGroup {
        group: "page_0".to_string(),
        sort_key: String::new(),
        files,
    };
    let request = Request {
        request_type: RequestType::SinglePage,
        groups: &[group],
        config: &state.config.book_layout_solver,
        ga_config: &state.config.page_layout_solver,
        canvas_config: &spread_config,
    };
    let result = run_solver(&request)?;
    if result.is_empty() {
        anyhow::bail!("Solver returned no result for cover page");
    }
    state.layout[0].slots = result[0].slots.clone();
    state.layout[0].photos = result[0].photos.clone();
    Ok(())
}

fn update_cover_structured(
    state: &mut crate::dto_models::ProjectState,
    files: Vec<PhotoFile>,
    photo_index: &HashMap<String, (PhotoFile, String)>,
) -> Result<()> {
    let cover = &state.config.book.cover;
    let mode = cover.mode;
    let inner_page_count = state.layout.len() - 1;

    warn_slot_count_mismatch(mode, files.len());

    let ratios: Vec<f64> = state.layout[0]
        .photos
        .iter()
        .filter_map(|id| photo_index.get(id))
        .map(|(f, _)| f.aspect_ratio())
        .collect();

    state.layout[0].slots = compute_cover_slots(cover, &ratios, inner_page_count)?;
    Ok(())
}

/// Splits the first `n` photos (flattened across groups) into cover files, returning
/// the cover files and the rebuilt remaining groups in their original order.
pub(super) fn split_cover_photos(
    groups: &[PhotoGroup],
    n: usize,
) -> (Vec<PhotoFile>, Vec<PhotoGroup>) {
    let mut flat: Vec<(PhotoFile, &str, &str)> = groups
        .iter()
        .flat_map(|g| {
            g.files
                .iter()
                .map(move |f| (f.clone(), g.group.as_str(), g.sort_key.as_str()))
        })
        .collect();

    let cover_files: Vec<PhotoFile> = flat.drain(..n.min(flat.len())).map(|(f, _, _)| f).collect();

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

/// Presents the full cover spread (front + back + spine) as `page_width_mm` to the GA solver.
struct CoverCanvasConfig<'a> {
    cover: &'a CoverConfig,
    inner_page_count: usize,
}

impl CanvasConfig for CoverCanvasConfig<'_> {
    fn page_width_mm(&self) -> f64 {
        self.cover.spread_width_mm(self.inner_page_count)
    }
    fn page_height_mm(&self) -> f64 {
        self.cover.height_mm
    }
    fn bleed_mm(&self) -> f64 {
        self.cover.bleed_mm
    }
    fn margin_mm(&self) -> f64 {
        self.cover.margin_mm
    }
    fn gap_mm(&self) -> f64 {
        self.cover.gap_mm
    }
    fn bleed_threshold_mm(&self) -> f64 {
        self.cover.bleed_threshold_mm
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
