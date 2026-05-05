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

/// Active drag activity: idle, pending confirmation, or confirmed dragging.
///
/// `Pending` holds the drag source but suppresses all visual effects until the
/// cursor moves more than `DRAG_THRESHOLD_PX` pixels. This ensures a short RMB
/// tap never flashes ghost overlays before the context menu opens.
///
/// State transitions:
///   Idle → Pending (on RMB press)
///   Pending → Dragging (on cursor move > threshold while RMB held)
///   Pending → Idle (on RMB release without sufficient movement → context menu)
///   Dragging → Idle (on RMB release → dispatch drag action)
#[derive(Default, Debug)]
pub enum ActiveDrag {
    #[default]
    Idle,
    /// RMB pressed but not yet moved enough to commit to a drag.
    Pending {
        source: DragSource,
        press_pos: egui::Pos2,
        press_instant: std::time::Instant,
    },
    Dragging(DragSource),
}

/// Minimum cursor movement in pixels to promote `Pending` → `Dragging`.
pub const DRAG_THRESHOLD_PX: f32 = 4.0;

impl ActiveDrag {
    /// Promote `Pending` → `Dragging` if the cursor has moved past the threshold.
    /// Returns `true` when a promotion occurred.
    pub fn maybe_promote(&mut self, cursor_now: egui::Pos2) -> bool {
        if let ActiveDrag::Pending {
            source, press_pos, ..
        } = self
            && cursor_now.distance(*press_pos) >= DRAG_THRESHOLD_PX
        {
            let source = std::mem::replace(source, DragSource::Pool { photo_ids: vec![] });
            *self = ActiveDrag::Dragging(source);
            return true;
        }
        false
    }

    /// Returns the contained `DragSource` if this is `Dragging`.
    pub fn dragging_source(&self) -> Option<&DragSource> {
        match self {
            ActiveDrag::Dragging(s) => Some(s),
            _ => None,
        }
    }
}

/// Unified drag state: current activity and persistent mode (Swap/Move).
#[derive(Default, Debug)]
pub struct DragState {
    pub active: ActiveDrag,
    pub mode: DragMode,
    pub manual: ManualDrag,
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

    #[test]
    fn pending_promotes_when_moved_past_threshold() {
        let mut active = ActiveDrag::Pending {
            source: DragSource::Pool { photo_ids: vec![] },
            press_pos: egui::pos2(0.0, 0.0),
            press_instant: std::time::Instant::now(),
        };
        // Below threshold: no promotion.
        let promoted = active.maybe_promote(egui::pos2(2.0, 0.0));
        assert!(!promoted);
        assert!(matches!(active, ActiveDrag::Pending { .. }));
        // Above threshold: promote.
        let promoted = active.maybe_promote(egui::pos2(10.0, 0.0));
        assert!(promoted);
        assert!(matches!(active, ActiveDrag::Dragging(_)));
    }

    #[test]
    fn pending_does_not_promote_below_threshold() {
        let mut active = ActiveDrag::Pending {
            source: DragSource::Pool { photo_ids: vec![] },
            press_pos: egui::pos2(0.0, 0.0),
            press_instant: std::time::Instant::now(),
        };
        let promoted = active.maybe_promote(egui::pos2(3.9, 0.0));
        assert!(!promoted);
    }
}
