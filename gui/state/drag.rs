/// Tracks an ongoing drag-and-drop gesture.
#[derive(Default)]
pub enum DragState {
    #[default]
    Idle,
    Dragging {
        src_page: usize,
        src_slot: usize,
        /// True when the M key is held → Move instead of Swap.
        is_move: bool,
        /// Screen position of the pointer when drag was initiated.
        /// Used to compute the grab offset for the ghost rectangle.
        cursor_at_drag_start: egui::Pos2,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_idle_by_default() {
        let d = DragState::default();
        assert!(matches!(d, DragState::Idle));
    }
}
