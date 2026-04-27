/// Commands that the UI dispatches to the background worker.
///
/// Constructed by `input_handler` and processed by `FotobuchApp::dispatch`.
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum PendingCommand {
    Swap {
        src_page: usize,
        src_slot: usize,
        dst_page: usize,
        dst_slot: usize,
    },
    Move {
        src_page: usize,
        /// All slots to move (from the selection if dragged slot is selected,
        /// otherwise just the dragged slot).
        src_slots: Vec<usize>,
        dst_page: usize,
    },
    Undo,
    Redo,
    Place {
        photo_ids: Vec<String>,
        dst_page: Option<usize>,
    },
    /// Nav-Drag: Seite ↔ Seite tauschen.
    PageSwap {
        left: usize,
        right: usize,
    },
    ConfigSet {
        key: String,
        value: String,
    },
    /// Move slots onto a newly inserted page.
    MoveToNewPage {
        src_page: usize,
        src_slots: Vec<usize>,
        at_position: usize,
    },
    /// Cross-page swap with contiguous slot ranges.
    SwapRange {
        src_page: usize,
        src_slots: Vec<usize>,
        dst_page: usize,
        dst_slots: Vec<usize>,
    },
    /// Remove photos from specified slots (photos become unplaced in the pool).
    Unplace {
        page: usize,
        slots: Vec<usize>,
    },
    /// Delete one or more entire pages; photos return to the pool as unplaced.
    DeletePages {
        pages: Vec<usize>,
    },
    /// Move an entire page to a new position (nav drag → `[+]` in Move mode).
    MovePage {
        src_page: usize,
        at_position: usize,
    },
    /// Rebuild auto-layout for the given pages.
    RebuildPages {
        pages: Vec<usize>,
    },
    /// Rebuild all auto-layout pages after user confirmation.
    RebuildAll,
    /// Release-quality PDF build.
    ReleaseBuild,
}
