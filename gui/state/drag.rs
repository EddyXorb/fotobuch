/// Returns true if two slot ratios (w/h) are considered similar (within 5 %).
pub fn slot_ratio_similar(ratio_a: f64, ratio_b: f64) -> bool {
    if ratio_b == 0.0 {
        return false;
    }
    (ratio_a / ratio_b - 1.0).abs() < 0.05
}

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

    #[test]
    fn ratio_same_returns_true() {
        assert!(slot_ratio_similar(1.5, 1.5));
        assert!(slot_ratio_similar(1.5, 1.52)); // within 5 %
    }

    #[test]
    fn ratio_different_returns_false() {
        assert!(!slot_ratio_similar(1.0, 2.0));
        assert!(!slot_ratio_similar(1.0, 1.06)); // just over 5 %
    }

    #[test]
    fn ratio_zero_denominator_returns_false() {
        assert!(!slot_ratio_similar(1.0, 0.0));
    }
}
