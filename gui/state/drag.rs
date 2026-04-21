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

/// Tracks an ongoing drag-and-drop gesture.
#[derive(Default)]
pub enum DragState {
    #[default]
    Idle,
    Dragging {
        src_page: usize,
        src_slot: usize,
        /// Screen position of the pointer when drag was initiated.
        /// Used to compute the grab offset for the ghost rectangle.
        cursor_at_drag_start: egui::Pos2,
    },
}

/// Drag innerhalb des Nav-Panels (Seite ↔ Seite tauschen).
#[derive(Default)]
pub enum NavDragState {
    #[default]
    Idle,
    Dragging {
        src_page: usize,
    },
}

/// Drag vom Foto-Pool auf eine Seite.
#[derive(Default)]
pub enum PoolDragState {
    #[default]
    Idle,
    /// `photo_ids` ist ein Snapshot der Selektion beim drag_started.
    Dragging { photo_ids: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_idle_by_default() {
        let d = DragState::default();
        assert!(matches!(d, DragState::Idle));
    }

    #[test]
    fn drag_mode_toggle() {
        assert_eq!(DragMode::Swap.toggle(), DragMode::Move);
        assert_eq!(DragMode::Move.toggle(), DragMode::Swap);
    }
}
