use super::{MultiSelection, PhotoSelection, SlotSelection};

/// Groups all independent selection states.
#[derive(Default)]
pub struct Selections {
    pub slots: SlotSelection,
    pub photos: PhotoSelection,
    /// Selected pages in the nav panel (multi-select).
    pub nav_pages: MultiSelection<usize>,
}
