/// Whether the current drag gesture is a Swap or a Move operation.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DragMode {
    #[default]
    Swap,
    Move,
}

impl DragMode {
    pub fn label(self) -> &'static str {
        match self {
            DragMode::Swap => "Swap",
            DragMode::Move => "Move",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            DragMode::Swap => DragMode::Move,
            DragMode::Move => DragMode::Swap,
        }
    }
}

/// What initiated the current drag gesture.
#[derive(Debug, Clone)]
pub enum DragSource {
    /// Right-mouse drag from a slot in the central panel.
    Slot {
        src_page: usize,
        src_slot: usize,
        /// Full slot selection captured at drag-start (may be multi-slot).
        src_slots: Vec<usize>,
        cursor_at_drag_start: egui::Pos2,
    },
    /// Right-mouse drag from a page thumbnail in the nav panel.
    NavPage {
        src_page: usize,
        /// All nav-selected pages captured at drag-start (analogous to `src_slots`).
        #[allow(dead_code)]
        src_pages: Vec<usize>,
    },
    /// Right-mouse drag from a photo row in the pool panel.
    Pool { photo_ids: Vec<String> },
}

/// Active drag activity: idle or dragging with a source.
#[derive(Default, Debug)]
pub enum ActiveDrag {
    #[default]
    Idle,
    Dragging(DragSource),
}

/// Unified drag state: current activity and persistent mode (Swap/Move).
#[derive(Default, Debug)]
pub struct DragState {
    pub active: ActiveDrag,
    pub mode: DragMode,
    pub manual: ManualDrag,
    /// When RMB was pressed (for tap-vs-drag discrimination).
    pub press_instant: Option<std::time::Instant>,
    /// Cursor position when RMB was pressed.
    pub press_pos: Option<egui::Pos2>,
}

/// Free-position drag on a Manual-mode page (RMB drag).
#[derive(Default, Debug, Clone)]
pub enum ManualDrag {
    #[default]
    Idle,
    Move {
        page: usize,
        slot: usize,
        pointer_origin: egui::Pos2,
        slot_origin_mm: (f64, f64),
    },
    Resize {
        page: usize,
        slot: usize,
        pointer_origin: egui::Pos2,
        /// x, y, w, h in mm at drag-start
        slot_origin_mm: (f64, f64, f64, f64),
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_idle_by_default() {
        let d = DragState::default();
        assert!(matches!(d.active, ActiveDrag::Idle));
    }

    #[test]
    fn drag_mode_toggle() {
        assert_eq!(DragMode::Swap.toggle(), DragMode::Move);
        assert_eq!(DragMode::Move.toggle(), DragMode::Swap);
    }
}
