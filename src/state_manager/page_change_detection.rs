//! Detection of outdated pages when project state changes.

use crate::dto_models::ProjectState;
use std::collections::{BTreeSet, HashMap, HashSet};

const ASPECT_RATIO_THRESHOLD: f64 = 0.001;

/// Computes which 1-based page numbers in `new` differ from `reference`.
///
/// A page is considered unchanged only if:
/// - No photo metadata changes (aspect ratio or area_weight)
/// - Page slot structure matches previous state exactly (position & dimensions)
/// - Each slot's aspect ratio matches its corresponding photo's aspect ratio
///
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
    let changed_photos = find_changed_photos(reference, new, &ref_photo_metadata);

    // Phase 2: Evaluate each new page
    for new_page in &new.layout {
        // 1. Check if any photo is in changed_photos
        if new_page.photos.iter().any(|id| changed_photos.contains(id)) {
            outdated.push(new_page.page);
            continue;
        }

        // 2. Find matching old page by slot structure
        let photo_set = new_page.photos.iter().cloned().collect::<BTreeSet<_>>();
        let candidate_indices = match page_hashes.get(&photo_set) {
            Some(indices) => indices,
            None => {
                // Page's photo set doesn't exist in reference
                outdated.push(new_page.page);
                continue;
            }
        };

        // Try to find a candidate with matching slot structure
        let matching_ref_page = candidate_indices.iter().find_map(|&idx| {
            let ref_page = &reference.layout[idx];
            if ref_page.slots == new_page.slots {
                Some(ref_page)
            } else {
                None
            }
        });

        if matching_ref_page.is_none() {
            outdated.push(new_page.page);
            continue;
        }

        // 3. Validate slot count matches photo count
        if new_page.slots.len() != new_page.photos.len() {
            outdated.push(new_page.page);
            continue;
        }

        // 4. Validate each slot's aspect ratio matches its photo's aspect ratio
        let mut page_valid = true;
        for (i, photo_id) in new_page.photos.iter().enumerate() {
            let slot_ar = new_page.slots[i].width_mm / new_page.slots[i].height_mm;

            // Get photo's aspect ratio from metadata map
            let photo_ar = match ref_photo_metadata.get(photo_id.as_str()) {
                Some((ar, _weight)) => *ar,
                None => {
                    // Photo not in reference metadata - conservative: mark as outdated
                    page_valid = false;
                    break;
                }
            };

            if (slot_ar - photo_ar).abs() > ASPECT_RATIO_THRESHOLD {
                page_valid = false;
                break;
            }
        }

        // If any AR validation failed, mark page as outdated
        if !page_valid {
            outdated.push(new_page.page);
            continue;
        }

        // If we reach here, page is unchanged
    }

    outdated
}

/// Build a map of photo ID -> (aspect_ratio, area_weight) from a photo collection.
fn build_photo_metadata(photos: &[crate::dto_models::PhotoGroup]) -> HashMap<String, (f64, f64)> {
    photos
        .iter()
        .flat_map(|group| {
            group.files.iter().map(|file| {
                (
                    file.id.clone(),
                    (file.aspect_ratio(), file.area_weight),
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
fn find_changed_photos(
    _reference: &ProjectState,
    new: &ProjectState,
    ref_metadata: &HashMap<String, (f64, f64)>,
) -> HashSet<String> {
    let new_metadata = build_photo_metadata(&new.photos);

    // Check for photos with changed metadata or new photos
    let mut changed = new_metadata
        .iter()
        .filter_map(|(photo_id, (new_ar, new_weight))| {
            if let Some((ref_ar, ref_weight)) = ref_metadata.get(photo_id) {
                // Check if aspect ratio or weight changed
                if (new_ar - ref_ar).abs() > ASPECT_RATIO_THRESHOLD
                    || (new_weight - ref_weight).abs() > ASPECT_RATIO_THRESHOLD
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{
        BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup, ProjectConfig,
        ProjectState, Slot,
    };

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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![2]); // Page 1 matches page1_ref (same slots), page 2 has different photos
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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![1]); // Slot AR (1.5) != photo AR (1.0)
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
        assert_eq!(outdated, vec![1]);
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
        assert_eq!(outdated, vec![1]); // No pages in reference
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
}
