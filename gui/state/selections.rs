use super::{PhotoSelection, SlotSelection};

/// Groups the two independent selection states: page slots and pool photos.
#[derive(Default)]
pub struct Selections {
    pub slots: SlotSelection,
    pub photos: PhotoSelection,
}
