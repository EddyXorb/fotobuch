use tracing::debug;

use crate::models::{BookConfig, Page, Photo, PhotoGroup, Placement};

/// Distributes all photos from the given groups across pages.
///
/// Strategy:
/// - Photos from the same group are kept together where possible.
/// - The number of photos per page is chosen based on count and orientation.
/// - Layout within a page is a simple grid/split that respects aspect ratios.
pub fn solve(groups: &[PhotoGroup], config: &BookConfig) -> Vec<Page> {
    let photos = collect_photos(groups);
    let chunks = split_into_pages(&photos, config.max_photos_per_page);

    chunks
        .into_iter()
        .map(|chunk| layout_page(chunk, config))
        .collect()
}

/// Flattens all groups into a single chronologically ordered list.
fn collect_photos(groups: &[PhotoGroup]) -> Vec<Photo> {
    groups
        .iter()
        .flat_map(|g| g.photos.iter().cloned())
        .collect()
}

/// Splits the photo list into page-sized chunks.
///
/// The chunk size is chosen per batch: a single landscape photo fills a page
/// alone; multiple photos are grouped up to `max_per_page`.
fn split_into_pages(photos: &[Photo], max_per_page: usize) -> Vec<Vec<Photo>> {
    let mut pages = Vec::new();
    let mut i = 0;

    while i < photos.len() {
        let remaining = &photos[i..];
        let count = pick_page_count(remaining, max_per_page);
        pages.push(remaining[..count].to_vec());
        i += count;
    }

    pages
}

/// Decides how many photos to place on the next page.
///
/// Rules:
/// - A single landscape photo with a very wide aspect ratio (> 2.0) gets its own page.
/// - Otherwise use `max_per_page`, but cap at `remaining.len()`.
fn pick_page_count(remaining: &[Photo], max_per_page: usize) -> usize {
    if remaining.is_empty() {
        return 0;
    }

    let first = &remaining[0];
    if first.aspect_ratio() > 2.0 {
        return 1; // Panorama – full page.
    }

    remaining.len().min(max_per_page)
}

/// Computes pixel-precise placements for a set of photos on one page.
///
/// Layout strategy by photo count:
/// - 1 photo  → full usable area
/// - 2 photos → side by side (or top/bottom for two portraits)
/// - 3 photos → one large left, two stacked right
/// - 4 photos → 2×2 grid
fn layout_page(photos: Vec<Photo>, config: &BookConfig) -> Page {
    let usable_w = config.page_width_mm - 2.0 * config.margin_mm;
    let usable_h = config.page_height_mm - 2.0 * config.margin_mm;
    let origin_x = config.margin_mm;
    let origin_y = config.margin_mm;
    let gap = config.gap_mm;

    debug!("Laying out {} photos on page ({} x {} mm usable)", photos.len(), usable_w, usable_h);

    let placements = match photos.len() {
        0 => vec![],
        1 => layout_single(&photos[0], origin_x, origin_y, usable_w, usable_h),
        2 => layout_two(&photos, origin_x, origin_y, usable_w, usable_h, gap),
        3 => layout_three(&photos, origin_x, origin_y, usable_w, usable_h, gap),
        _ => layout_grid(&photos[..4], origin_x, origin_y, usable_w, usable_h, gap),
    };

    Page { placements }
}

fn layout_single(photo: &Photo, x: f64, y: f64, w: f64, h: f64) -> Vec<Placement> {
    vec![Placement {
        photo: photo.clone(),
        x_mm: x,
        y_mm: y,
        width_mm: w,
        height_mm: h,
    }]
}

/// Two photos: side-by-side if both landscape, stacked if both portrait.
fn layout_two(photos: &[Photo], x: f64, y: f64, w: f64, h: f64, gap: f64) -> Vec<Placement> {
    let both_portrait = photos.iter().all(|p| !p.is_landscape());

    if both_portrait {
        // Stack vertically.
        let cell_h = (h - gap) / 2.0;
        vec![
            make_placement(&photos[0], x, y, w, cell_h),
            make_placement(&photos[1], x, y + cell_h + gap, w, cell_h),
        ]
    } else {
        // Side by side.
        let cell_w = (w - gap) / 2.0;
        vec![
            make_placement(&photos[0], x, y, cell_w, h),
            make_placement(&photos[1], x + cell_w + gap, y, cell_w, h),
        ]
    }
}

/// Three photos: one large on the left, two stacked on the right.
fn layout_three(photos: &[Photo], x: f64, y: f64, w: f64, h: f64, gap: f64) -> Vec<Placement> {
    let left_w = (w - gap) * 0.6;
    let right_w = w - left_w - gap;
    let cell_h = (h - gap) / 2.0;

    vec![
        make_placement(&photos[0], x, y, left_w, h),
        make_placement(&photos[1], x + left_w + gap, y, right_w, cell_h),
        make_placement(&photos[2], x + left_w + gap, y + cell_h + gap, right_w, cell_h),
    ]
}

/// Four photos in a 2×2 grid.
fn layout_grid(photos: &[Photo], x: f64, y: f64, w: f64, h: f64, gap: f64) -> Vec<Placement> {
    let cell_w = (w - gap) / 2.0;
    let cell_h = (h - gap) / 2.0;

    vec![
        make_placement(&photos[0], x, y, cell_w, cell_h),
        make_placement(&photos[1], x + cell_w + gap, y, cell_w, cell_h),
        make_placement(&photos[2], x, y + cell_h + gap, cell_w, cell_h),
        make_placement(&photos[3], x + cell_w + gap, y + cell_h + gap, cell_w, cell_h),
    ]
}

fn make_placement(photo: &Photo, x: f64, y: f64, w: f64, h: f64) -> Placement {
    Placement {
        photo: photo.clone(),
        x_mm: x,
        y_mm: y,
        width_mm: w,
        height_mm: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::models::Photo;

    fn dummy_photo(landscape: bool) -> Photo {
        let mut p = Photo::new(PathBuf::from("test.jpg"));
        p.dimensions = Some(if landscape { (4000, 3000) } else { (3000, 4000) });
        p
    }

    #[test]
    fn test_layout_single_fills_page() {
        let config = BookConfig::default();
        let photos = vec![dummy_photo(true)];
        let page = layout_page(photos, &config);
        assert_eq!(page.placements.len(), 1);
        let p = &page.placements[0];
        assert_eq!(p.x_mm, config.margin_mm);
        assert_eq!(p.y_mm, config.margin_mm);
    }

    #[test]
    fn test_layout_two_side_by_side() {
        let config = BookConfig::default();
        let photos = vec![dummy_photo(true), dummy_photo(true)];
        let page = layout_page(photos, &config);
        assert_eq!(page.placements.len(), 2);
        // Second photo should be to the right.
        assert!(page.placements[1].x_mm > page.placements[0].x_mm);
    }

    #[test]
    fn test_split_into_pages_respects_max() {
        let photos: Vec<Photo> = (0..10).map(|_| dummy_photo(true)).collect();
        let chunks = split_into_pages(&photos, 4);
        assert!(chunks.iter().all(|c| c.len() <= 4));
        assert_eq!(chunks.iter().map(|c| c.len()).sum::<usize>(), 10);
    }
}
