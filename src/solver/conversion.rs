//! Conversion boundary between DTO types and solver data models.
//!
//! This module is the single place where domain-ordering logic (sort by timestamp)
//! and DTO ↔ solver-model bridges live. Data models themselves stay logic-free.
mod output_transform;

use crate::dto_models::{CanvasConfig, LayoutPage, PageMode, PhotoGroup, Slot};
use crate::solver::data_models::{Photo, PhotoPlacement, SolverPageLayout};

/// Converts photo groups to a flat, timestamp-sorted list of Photos.
///
/// Sorts each group's files by timestamp before flattening, so the solver
/// always receives photos in chronological order within each group.
pub(crate) fn photos_from_groups(groups: &[PhotoGroup]) -> Vec<Photo> {
    let mut groups_copy = groups.to_vec();
    for group in &mut groups_copy {
        group.files.sort_by_key(|a| a.timestamp);
    }
    groups_copy
        .iter()
        .flat_map(|group| {
            group
                .files
                .iter()
                .map(|file| Photo::from_photo_file(file, &group.group))
        })
        .collect()
}

/// Converts a solver page layout to a DTO [`LayoutPage`].
///
/// Centers the layout on the canvas and applies bleed scaling before mapping
/// photo indices to IDs and positions to [`Slot`]s.
pub(crate) fn to_layout_page(
    layout: &SolverPageLayout,
    page_num: usize,
    photos: &[Photo],
    canvas_config: &impl CanvasConfig,
) -> LayoutPage {
    let centered = layout.centered();
    let adapted = output_transform::zoom_to_respect_bleed(&centered, canvas_config);

    let photo_ids: Vec<String> = adapted
        .placements
        .iter()
        .map(|p| photos[p.photo_idx as usize].id.clone())
        .collect();

    let slots: Vec<Slot> = adapted.placements.iter().map(slot_from_placement).collect();

    LayoutPage {
        page: page_num,
        photos: photo_ids,
        slots,
        mode: PageMode::Auto,
    }
}

fn slot_from_placement(p: &PhotoPlacement) -> Slot {
    Slot {
        x_mm: p.x,
        y_mm: p.y,
        width_mm: p.w,
        height_mm: p.h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::BookConfig;
    use crate::solver::data_models::{Canvas, PhotoPlacement, SolverPageLayout};
    use approx::assert_relative_eq;

    fn make_photo(id: &str) -> Photo {
        Photo::new(id.to_string(), 1.5, 1.0, "g".to_string())
    }

    #[test]
    fn to_layout_page_empty() {
        let canvas = Canvas::new(200.0, 200.0, 2.0);
        let layout = SolverPageLayout::new(vec![], canvas);
        let dto = to_layout_page(&layout, 1, &[], &BookConfig::default());
        assert_eq!(dto.page, 1);
        assert!(dto.photos.is_empty());
        assert!(dto.slots.is_empty());
    }

    #[test]
    fn to_layout_page_single_photo() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 10.0, 20.0, 100.0, 80.0)];
        let layout = SolverPageLayout::new(placements, canvas);
        let photos = vec![make_photo("photo_abc")];

        let book_config = BookConfig {
            bleed_mm: 0.0,
            ..BookConfig::default()
        };
        let dto = to_layout_page(&layout, 2, &photos, &book_config);

        assert_eq!(dto.page, 2);
        assert_eq!(dto.photos, vec!["photo_abc"]);
        // After centering: offset_x = (200-100)/2 - 10 = 40 → x=50; offset_y = (200-80)/2 - 20 = 40 → y=60
        assert_relative_eq!(dto.slots[0].x_mm, 50.0, epsilon = 1e-6);
        assert_relative_eq!(dto.slots[0].y_mm, 60.0, epsilon = 1e-6);
        assert_relative_eq!(dto.slots[0].width_mm, 100.0, epsilon = 1e-6);
        assert_relative_eq!(dto.slots[0].height_mm, 80.0, epsilon = 1e-6);
    }

    #[test]
    fn to_layout_page_no_bleed_with_margin() {
        let canvas = Canvas::new(200.0, 200.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 50.0, 50.0, 100.0, 100.0)];
        let layout = SolverPageLayout::new(placements, canvas);
        let photos = vec![make_photo("photo_abc")];

        let book_config = BookConfig {
            bleed_mm: 5.0,
            margin_mm: 10.0,
            bleed_threshold_mm: 5.0,
            ..BookConfig::default()
        };
        let dto = to_layout_page(&layout, 1, &photos, &book_config);

        assert_relative_eq!(dto.slots[0].x_mm, 50.0, epsilon = 1e-6);
        assert_relative_eq!(dto.slots[0].y_mm, 50.0, epsilon = 1e-6);
    }
}
