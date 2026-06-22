//! Writing the selected photos into the layout (per destination).

use std::collections::HashSet;

use crate::models::{LayoutPage, PageMode, build_photo_index};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

use super::PlaceDst;
use super::page_placement::{UnplacedPhoto, place_chronologically};

/// Place the filtered photos according to `dst`.
/// Returns `(pages_affected, pages_inserted)` (both 0-based).
pub(super) fn place_photos(
    s: &mut WriteLayoutState,
    dst: &PlaceDst,
    filtered: &[&UnplacedPhoto],
) -> (Vec<usize>, Vec<usize>) {
    match dst {
        PlaceDst::NewPageAt(pos) => place_into_new_page(s.layout_mut(), filtered, *pos),
        PlaceDst::Page(page) => (place_into_page(s.layout_mut(), filtered, *page), vec![]),
        PlaceDst::Auto => place_chronological(s, filtered),
    }
}

/// Places all photos onto a specific page.
/// Returns the affected page index (0-based, as a single-element vector).
fn place_into_page(
    layout: &mut [LayoutPage],
    photos: &[&UnplacedPhoto],
    page_idx: usize,
) -> Vec<usize> {
    for photo in photos {
        layout[page_idx].photos.push(photo.id.clone());
    }
    vec![page_idx]
}

/// Creates a new page at the given position and places all photos there.
/// Returns `(affected pages, inserted pages)`.
fn place_into_new_page(
    layout: &mut Vec<LayoutPage>,
    photos: &[&UnplacedPhoto],
    position: usize,
) -> (Vec<usize>, Vec<usize>) {
    let photo_ids: Vec<String> = photos.iter().map(|p| p.id.clone()).collect();
    layout.insert(
        position,
        LayoutPage {
            photos: photo_ids,
            slots: vec![],
            mode: PageMode::Auto,
        },
    );
    (vec![position], vec![position])
}

/// Distributes photos chronologically across existing pages.
fn place_chronological(
    s: &mut WriteLayoutState,
    filtered: &[&UnplacedPhoto],
) -> (Vec<usize>, Vec<usize>) {
    let photo_index = build_photo_index(s.photos());
    let cover_active = s.config().book.cover.active;
    let assignments = place_chronologically(s.layout(), &photo_index, cover_active, filtered);

    let mut affected = HashSet::new();
    for (page_idx, photo_id) in assignments {
        s.layout_mut()[page_idx].photos.push(photo_id);
        affected.insert(page_idx);
    }
    let mut result: Vec<usize> = affected.into_iter().collect();
    result.sort();
    (result, vec![])
}
