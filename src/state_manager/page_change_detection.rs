//! Detection of outdated pages when project state changes.

use crate::dto_models::{ProjectState, SpineConfig};
use std::collections::{BTreeSet, HashMap, HashSet};

const ASPECT_RATIO_THRESHOLD: f64 = 0.001;
const WEIGHT_THRESHOLD: f64 = 0.01;

/// Metadata for a single photo used in change detection.
struct PhotoMetadata {
    aspect_ratio: f64,
    area_weight: f64,
}

/// Computes which pages (by array index in `new.layout`) differ from `reference`.
///
/// A page is considered unchanged only if:
/// - No photo metadata changes (aspect ratio or area_weight)
/// - Page slot structure matches previous state exactly (position & dimensions)
/// - Each slot's aspect ratio matches its corresponding photo's aspect ratio
///
/// Returns 0-based indices of pages in the `new.layout` array that are outdated.
/// It does not matter if pages are reordered in position, as long as the above
/// constraints are met.
pub fn compute_outdated_pages(reference: &ProjectState, new: &ProjectState) -> Vec<usize> {
    let mut outdated = Vec::new();

    // Phase 1: Build reference maps

    // Map photo IDs to (aspect_ratio, area_weight)
    let ref_photo_metadata = build_photo_metadata(&reference.photos);

    // Build page_hashes: BTreeSet of photo IDs -> Vec of indices in reference.layout
    let page_hashes = build_page_hashes(&reference.layout);

    // Find photos with changed metadata
    let changed_photos = find_metadata_changed_photos(reference, new, &ref_photo_metadata);

    let cover = &new.config.book.cover;
    let cover_skips_ar = cover.active && cover.mode.allows_ar_mismatch();
    let inner_canvas_outdated = inner_canvas_changed(reference, new);
    let cover_canvas_outdated = cover.active && cover_canvas_changed(reference, new);

    // Phase 2: Evaluate each new page
    for (page_index, new_page) in new.layout.iter().enumerate() {
        let is_cover = cover.active && page_index == 0;
        let canvas_outdated = if is_cover {
            cover_canvas_outdated
        } else {
            inner_canvas_outdated
        };
        let skip_ar_check = cover_skips_ar && is_cover;
        if canvas_outdated
            || page_is_outdated_by_metadata(&changed_photos, new_page)
            || page_is_outdated_by_slot_structure(new_page, &reference.layout, &page_hashes)
            || (!skip_ar_check
                && page_is_outdated_by_aspect_ratio_violation(new_page, &ref_photo_metadata))
        {
            outdated.push(page_index);
        }
    }

    outdated
}

fn page_is_outdated_by_metadata(
    changed_photos: &HashSet<String>,
    new_page: &crate::dto_models::LayoutPage,
) -> bool {
    new_page.photos.iter().any(|id| changed_photos.contains(id))
}

fn page_is_outdated_by_slot_structure(
    new_page: &crate::dto_models::LayoutPage,
    reference_layout: &[crate::dto_models::LayoutPage],
    page_hashes: &HashMap<BTreeSet<String>, Vec<usize>>,
) -> bool {
    let photo_set = new_page.photos.iter().cloned().collect::<BTreeSet<_>>();
    let candidate_indices = match page_hashes.get(&photo_set) {
        Some(indices) => indices,
        None => return true,
    };

    let found_match = candidate_indices
        .iter()
        .any(|&idx| reference_layout[idx].slots == new_page.slots);

    if !found_match {
        return true;
    }

    new_page.slots.len() != new_page.photos.len()
}

fn page_is_outdated_by_aspect_ratio_violation(
    new_page: &crate::dto_models::LayoutPage,
    ref_photo_metadata: &HashMap<String, PhotoMetadata>,
) -> bool {
    new_page.photos.iter().enumerate().any(|(i, photo_id)| {
        let slot_ar = new_page.slots[i].width_mm / new_page.slots[i].height_mm;
        match ref_photo_metadata.get(photo_id.as_str()) {
            Some(meta) => (slot_ar - meta.aspect_ratio).abs() > ASPECT_RATIO_THRESHOLD,
            None => true,
        }
    })
}

/// Build a map of photo ID -> metadata from a photo collection.
fn build_photo_metadata(
    photos: &[crate::dto_models::PhotoGroup],
) -> HashMap<String, PhotoMetadata> {
    photos
        .iter()
        .flat_map(|group| {
            group.files.iter().map(|file| {
                (
                    file.id.clone(),
                    PhotoMetadata {
                        aspect_ratio: file.aspect_ratio(),
                        area_weight: file.area_weight,
                    },
                )
            })
        })
        .collect()
}

/// Build page_hashes: BTreeSet of photo IDs -> Vec of indices in layout where this set appears.
fn build_page_hashes(
    layout: &[crate::dto_models::LayoutPage],
) -> HashMap<BTreeSet<String>, Vec<usize>> {
    let mut map: HashMap<BTreeSet<String>, Vec<usize>> = HashMap::new();
    for (idx, page) in layout.iter().enumerate() {
        let photo_set: BTreeSet<String> = page.photos.iter().cloned().collect();
        map.entry(photo_set).or_default().push(idx);
    }
    map
}

/// Find photo IDs whose metadata changed between reference and new state.
/// Includes newly added photos and removed photos.
fn find_metadata_changed_photos(
    _reference: &ProjectState,
    new: &ProjectState,
    ref_metadata: &HashMap<String, PhotoMetadata>,
) -> HashSet<String> {
    let new_metadata = build_photo_metadata(&new.photos);

    // Check for photos with changed metadata or new photos
    let mut changed = new_metadata
        .iter()
        .filter_map(|(photo_id, new_meta)| {
            if let Some(ref_meta) = ref_metadata.get(photo_id) {
                // Check if aspect ratio or weight changed
                if (new_meta.aspect_ratio - ref_meta.aspect_ratio).abs() > ASPECT_RATIO_THRESHOLD
                    || (new_meta.area_weight - ref_meta.area_weight).abs() > WEIGHT_THRESHOLD
                {
                    Some(photo_id.clone())
                } else {
                    None
                }
            } else {
                // Photo exists in new but not in reference (new photo)
                Some(photo_id.clone())
            }
        })
        .collect::<HashSet<_>>();

    // Check for removed photos (exist in reference but not in new)
    for photo_id in ref_metadata.keys() {
        if !new_metadata.contains_key(photo_id) {
            changed.insert(photo_id.clone());
        }
    }

    changed
}

fn inner_canvas_changed(reference: &ProjectState, new: &ProjectState) -> bool {
    let r = &reference.config.book;
    let n = &new.config.book;
    r.page_width_mm != n.page_width_mm
        || r.page_height_mm != n.page_height_mm
        || r.bleed_mm != n.bleed_mm
        || r.margin_mm != n.margin_mm
        || r.gap_mm != n.gap_mm
        || r.bleed_threshold_mm != n.bleed_threshold_mm
}

fn inner_page_count(state: &ProjectState) -> usize {
    if state.config.book.cover.active {
        state.layout.len().saturating_sub(1)
    } else {
        state.layout.len()
    }
}

fn cover_canvas_changed(reference: &ProjectState, new: &ProjectState) -> bool {
    let r = &reference.config.book.cover;
    let n = &new.config.book.cover;
    let inner_count_changed = inner_page_count(reference) != inner_page_count(new);
    r.height_mm != n.height_mm
        || r.front_back_width_mm != n.front_back_width_mm
        || r.bleed_mm != n.bleed_mm
        || r.margin_mm != n.margin_mm
        || r.gap_mm != n.gap_mm
        || r.bleed_threshold_mm != n.bleed_threshold_mm
        || r.mode != n.mode
        || spine_changed(&r.spine, &n.spine, inner_count_changed)
}

fn spine_changed(r: &SpineConfig, n: &SpineConfig, inner_count_changed: bool) -> bool {
    match (r, n) {
        (
            SpineConfig::Auto {
                spine_mm_per_10_pages: r_rate,
            },
            SpineConfig::Auto {
                spine_mm_per_10_pages: n_rate,
            },
        ) => r_rate != n_rate || inner_count_changed,
        (
            SpineConfig::Fixed {
                spine_width_mm: r_w,
            },
            SpineConfig::Fixed {
                spine_width_mm: n_w,
            },
        ) => r_w != n_w,
        _ => true, // variant changed (Auto ↔ Fixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{
        BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup, ProjectConfig,
        ProjectState, Slot,
    };
    use crate::dto_models::{CoverConfig, CoverMode};

    fn make_state(title: &str, pages: Vec<LayoutPage>) -> ProjectState {
        ProjectState {
            config: ProjectConfig {
                book: BookConfig {
                    title: title.to_owned(),
                    page_width_mm: 420.0,
                    page_height_mm: 297.0,
                    bleed_mm: 3.0,
                    margin_mm: 10.0,
                    gap_mm: 5.0,
                    bleed_threshold_mm: 3.0,
                    dpi: 300.0,
                    cover: Default::default(),
                    appendix: Default::default(),
                },
                page_layout_solver: Default::default(),
                preview: Default::default(),
                book_layout_solver: BookLayoutSolverConfig::default(),
            },
            photos: vec![],
            layout: pages,
        }
    }

    fn make_photo(id: &str, width: u32, height: u32, weight: f64) -> PhotoFile {
        PhotoFile {
            id: id.to_owned(),
            source: format!("/photos/{}.jpg", id),
            width_px: width,
            height_px: height,
            area_weight: weight,
            timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
            hash: String::new(),
        }
    }

    fn make_slot(x: f64, y: f64, w: f64, h: f64) -> Slot {
        Slot {
            x_mm: x,
            y_mm: y,
            width_mm: w,
            height_mm: h,
        }
    }

    fn make_page(page_num: usize, photos: Vec<String>, slots: Vec<Slot>) -> LayoutPage {
        LayoutPage {
            page: page_num,
            photos,
            slots,
        }
    }

    #[test]
    fn test_unchanged_pages() {
        let page = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );

        let mut reference = make_state("ref", vec![page.clone()]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
            ],
        }];

        let mut new = make_state("new", vec![page.clone()]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
            ],
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, Vec::<usize>::new());
    }

    #[test]
    fn test_photo_metadata_changed_aspect_ratio() {
        let page_ref = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );

        let mut reference = make_state("ref", vec![page_ref]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)],
        }];

        let mut new = make_state("new", vec![page_new]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 120, 100, 1.0)], // AR changed from 1.0 to 1.2
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 in new.layout
    }

    #[test]
    fn test_photo_metadata_changed_weight() {
        let page_ref = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );

        let mut reference = make_state("ref", vec![page_ref]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)],
        }];

        let mut new = make_state("new", vec![page_new]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.5)], // Weight changed from 1.0 to 1.5
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 in new.layout
    }

    #[test]
    fn test_slot_position_changed() {
        let page_ref = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(10.0, 0.0, 100.0, 100.0)], // x changed
        );

        let reference = make_state("ref", vec![page_ref]);
        let new = make_state("new", vec![page_new]);

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 in new.layout
    }

    #[test]
    fn test_slot_dimensions_changed() {
        let page_ref = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 110.0, 100.0)], // width changed
        );

        let reference = make_state("ref", vec![page_ref]);
        let new = make_state("new", vec![page_new]);

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 in new.layout
    }

    #[test]
    fn test_photo_list_changed() {
        let page_ref = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()], // B removed
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );

        let reference = make_state("ref", vec![page_ref]);
        let new = make_state("new", vec![page_new]);

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 in new.layout
    }

    #[test]
    fn test_page_reordering() {
        let page1_ref = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );
        let page2_ref = make_page(
            2,
            vec!["C".to_string(), "D".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );

        let page1_new = make_page(
            1,
            vec!["C".to_string(), "D".to_string()], // Swapped content
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );
        let page2_new = make_page(
            2,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );

        let mut reference = make_state("ref", vec![page1_ref, page2_ref]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
                make_photo("C", 100, 100, 1.0),
                make_photo("D", 100, 100, 1.0),
            ],
        }];

        let mut new = make_state("new", vec![page1_new, page2_new]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
                make_photo("C", 100, 100, 1.0),
                make_photo("D", 100, 100, 1.0),
            ],
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, Vec::<usize>::new()); // Both pages unchanged (just reordered)
    }

    #[test]
    fn test_duplicate_photo_sets_in_reference() {
        // Reference has two pages with same photos but different slots
        let page1_ref = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );
        let page2_ref = make_page(
            2,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 150.0, 100.0),
                make_slot(150.0, 0.0, 100.0, 100.0),
            ],
        );

        // New has one page matching page1's slots, one page with different photos
        let page1_new = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );
        let page2_new = make_page(
            2,
            vec!["C".to_string(), "D".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );

        let mut reference = make_state("ref", vec![page1_ref, page2_ref]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
                make_photo("C", 100, 100, 1.0),
                make_photo("D", 100, 100, 1.0),
            ],
        }];

        let mut new = make_state("new", vec![page1_new, page2_new]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
                make_photo("C", 100, 100, 1.0),
                make_photo("D", 100, 100, 1.0),
            ],
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![1]); // Index 1 (second page) has different photos
    }

    #[test]
    fn test_slot_count_mismatch() {
        let page_ref = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![
                make_slot(0.0, 0.0, 100.0, 100.0),
                make_slot(100.0, 0.0, 100.0, 100.0),
            ],
        );
        let page_new = make_page(
            1,
            vec!["A".to_string(), "B".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)], // Only 1 slot for 2 photos
        );

        let reference = make_state("ref", vec![page_ref]);
        let new = make_state("new", vec![page_new]);

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 in new.layout
    }

    #[test]
    fn test_aspect_ratio_mismatch() {
        let page_ref = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)], // AR = 1.0
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 150.0, 100.0)], // AR = 1.5
        );

        let mut reference = make_state("ref", vec![page_ref]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)], // Photo AR = 1.0
        }];

        let mut new = make_state("new", vec![page_new]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)],
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 - Slot AR (1.5) != photo AR (1.0)
    }

    #[test]
    fn test_aspect_ratio_within_tolerance() {
        let page_ref = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)], // AR = 1.0
        );
        let page_new = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0005, 100.0)], // AR ≈ 1.0000050
        );

        let mut reference = make_state("ref", vec![page_ref]);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)],
        }];

        let mut new = make_state("new", vec![page_new]);
        new.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)],
        }];

        let outdated = compute_outdated_pages(&reference, &new);
        // Slot AR differs, should be outdated (slots don't match exactly)
        assert_eq!(outdated, vec![0]); // Index 0
    }

    #[test]
    fn test_empty_reference() {
        let page = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );

        let reference = make_state("ref", vec![]);
        let new = make_state("new", vec![page]);

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // Index 0 - No pages in reference
    }

    #[test]
    fn test_empty_new() {
        let page = make_page(
            1,
            vec!["A".to_string()],
            vec![make_slot(0.0, 0.0, 100.0, 100.0)],
        );

        let reference = make_state("ref", vec![page]);
        let new = make_state("new", vec![]);

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, Vec::<usize>::new()); // No pages to check
    }

    fn make_state_with_cover(
        title: &str,
        pages: Vec<LayoutPage>,
        cover_mode: CoverMode,
    ) -> ProjectState {
        let mut state = make_state(title, pages);
        state.config.book.cover = CoverConfig {
            active: true,
            mode: cover_mode,
            ..CoverConfig::default()
        };
        state
    }

    #[test]
    fn test_cover_full_mode_ignores_ar_mismatch_on_cover_page() {
        // In FrontFull the cover solver fills the slot to canvas dimensions (crops).
        // Slot AR = 1.5, photo AR = 1.0 — mismatch is intentional and must not trigger re-solve.
        // Slots are identical in ref and new (canvas unchanged), so slot-structure check passes.
        let cover_slot = make_slot(0.0, 0.0, 150.0, 100.0); // AR = 1.5
        let cover_ref = make_page(0, vec!["A".to_string()], vec![cover_slot.clone()]);
        let cover_new = make_page(0, vec!["A".to_string()], vec![cover_slot]);

        let mut reference = make_state_with_cover("ref", vec![cover_ref], CoverMode::FrontFull);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)], // photo AR = 1.0
        }];

        let mut new = make_state_with_cover("new", vec![cover_new], CoverMode::FrontFull);
        new.photos = reference.photos.clone();

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, Vec::<usize>::new()); // AR mismatch tolerated in FrontFull
    }

    #[test]
    fn test_cover_non_full_mode_still_checks_ar() {
        // Front mode preserves AR — slot AR mismatch with photo must still trigger re-solve.
        let cover_slot = make_slot(0.0, 0.0, 150.0, 100.0); // AR = 1.5
        let cover_ref = make_page(0, vec!["A".to_string()], vec![cover_slot.clone()]);
        let cover_new = make_page(0, vec!["A".to_string()], vec![cover_slot]);

        let mut reference = make_state_with_cover("ref", vec![cover_ref], CoverMode::Front);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)], // photo AR = 1.0
        }];

        let mut new = make_state_with_cover("new", vec![cover_new], CoverMode::Front);
        new.photos = reference.photos.clone();

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // AR mismatch still triggers re-solve
    }

    #[test]
    fn test_cover_full_mode_ar_skip_does_not_affect_inner_pages() {
        // Cover AR mismatch is tolerated (FrontFull), inner page AR mismatch must still trigger.
        let cover_slot = make_slot(0.0, 0.0, 150.0, 100.0); // AR = 1.5, photo AR = 1.0 — OK
        let inner_slot = make_slot(0.0, 0.0, 150.0, 100.0); // AR = 1.5, photo AR = 1.0 — NOT OK
        let cover_ref = make_page(0, vec!["A".to_string()], vec![cover_slot.clone()]);
        let cover_new = make_page(0, vec!["A".to_string()], vec![cover_slot]);
        let inner_ref = make_page(1, vec!["B".to_string()], vec![inner_slot.clone()]);
        let inner_new = make_page(1, vec!["B".to_string()], vec![inner_slot]);

        let mut reference =
            make_state_with_cover("ref", vec![cover_ref, inner_ref], CoverMode::FrontFull);
        reference.photos = vec![PhotoGroup {
            group: "test".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
            ],
        }];

        let mut new =
            make_state_with_cover("new", vec![cover_new, inner_new], CoverMode::FrontFull);
        new.photos = reference.photos.clone();

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![1]); // Only inner page is outdated
    }

    #[test]
    fn test_inner_canvas_change_marks_all_inner_pages_outdated() {
        let slot = make_slot(0.0, 0.0, 100.0, 100.0);
        let pages = vec![
            make_page(0, vec!["A".to_string()], vec![slot.clone()]),
            make_page(1, vec!["B".to_string()], vec![slot.clone()]),
        ];

        let mut reference = make_state("ref", pages.clone());
        reference.photos = vec![PhotoGroup {
            group: "g".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
            ],
        }];
        let mut new = make_state("new", pages);
        new.photos = reference.photos.clone();
        new.config.book.page_width_mm = 500.0; // canvas changed

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0, 1]);
    }

    #[test]
    fn test_cover_canvas_change_marks_only_cover_outdated() {
        let slot = make_slot(0.0, 0.0, 100.0, 100.0);
        let cover_page = make_page(0, vec!["A".to_string()], vec![slot.clone()]);
        let inner_page = make_page(1, vec!["B".to_string()], vec![slot.clone()]);

        let mut reference = make_state_with_cover(
            "ref",
            vec![cover_page.clone(), inner_page.clone()],
            CoverMode::Free,
        );
        reference.photos = vec![PhotoGroup {
            group: "g".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
            ],
        }];
        let mut new = make_state_with_cover("new", vec![cover_page, inner_page], CoverMode::Free);
        new.photos = reference.photos.clone();
        new.config.book.cover.height_mm = 350.0; // cover canvas changed

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]); // only cover page
    }

    #[test]
    fn test_spine_rate_change_marks_cover_outdated() {
        use crate::dto_models::SpineConfig;
        let slot = make_slot(0.0, 0.0, 100.0, 100.0);
        let cover_page = make_page(0, vec!["A".to_string()], vec![slot.clone()]);

        let mut reference = make_state_with_cover("ref", vec![cover_page.clone()], CoverMode::Free);
        reference.photos = vec![PhotoGroup {
            group: "g".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![make_photo("A", 100, 100, 1.0)],
        }];
        let mut new = make_state_with_cover("new", vec![cover_page], CoverMode::Free);
        new.photos = reference.photos.clone();
        new.config.book.cover.spine = SpineConfig::Auto {
            spine_mm_per_10_pages: 2.0,
        };

        let outdated = compute_outdated_pages(&reference, &new);
        assert_eq!(outdated, vec![0]);
    }

    #[test]
    fn test_inner_page_count_change_marks_cover_outdated_in_auto_spine() {
        // Auto spine: more inner pages → wider spine → cover canvas changed.
        use crate::dto_models::SpineConfig;
        let slot = make_slot(0.0, 0.0, 100.0, 100.0);
        let cover_page = make_page(0, vec!["A".to_string()], vec![slot.clone()]);
        let inner1 = make_page(1, vec!["B".to_string()], vec![slot.clone()]);
        let inner2 = make_page(2, vec!["C".to_string()], vec![slot.clone()]);

        let mut reference = make_state_with_cover(
            "ref",
            vec![cover_page.clone(), inner1.clone()],
            CoverMode::Free,
        );
        reference.config.book.cover.spine = SpineConfig::Auto {
            spine_mm_per_10_pages: 1.4,
        };
        reference.photos = vec![PhotoGroup {
            group: "g".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 100, 100, 1.0),
                make_photo("C", 100, 100, 1.0),
            ],
        }];

        let mut new =
            make_state_with_cover("new", vec![cover_page, inner1, inner2], CoverMode::Free);
        new.config.book.cover.spine = SpineConfig::Auto {
            spine_mm_per_10_pages: 1.4,
        };
        new.photos = reference.photos.clone();

        let outdated = compute_outdated_pages(&reference, &new);
        assert!(
            outdated.contains(&0),
            "cover must be outdated when inner page count changes with Auto spine"
        );
    }

    #[test]
    fn test_inner_page_count_change_does_not_affect_fixed_spine() {
        // Fixed spine: page count is irrelevant, cover width stays the same.
        use crate::dto_models::SpineConfig;
        let slot = make_slot(0.0, 0.0, 150.0, 100.0); // AR=1.5 — FrontFull crops, so AR skip applies
        let cover_page = make_page(0, vec!["A".to_string()], vec![slot.clone()]);
        let inner1 = make_page(1, vec!["B".to_string()], vec![slot.clone()]);
        let inner2 = make_page(2, vec!["C".to_string()], vec![slot.clone()]);

        let mut reference = make_state_with_cover(
            "ref",
            vec![cover_page.clone(), inner1.clone()],
            CoverMode::FrontFull,
        );
        reference.config.book.cover.spine = SpineConfig::Fixed {
            spine_width_mm: 3.0,
        };
        reference.photos = vec![PhotoGroup {
            group: "g".to_string(),
            sort_key: "2024-01-01T00:00:00Z".to_string(),
            files: vec![
                make_photo("A", 100, 100, 1.0),
                make_photo("B", 150, 100, 1.0),
                make_photo("C", 150, 100, 1.0),
            ],
        }];

        let mut new = make_state_with_cover(
            "new",
            vec![cover_page, inner1, inner2],
            CoverMode::FrontFull,
        );
        new.config.book.cover.spine = SpineConfig::Fixed {
            spine_width_mm: 3.0,
        };
        new.photos = reference.photos.clone();

        let outdated = compute_outdated_pages(&reference, &new);
        assert!(
            !outdated.contains(&0),
            "cover must not be outdated when inner page count changes with Fixed spine"
        );
    }
}
