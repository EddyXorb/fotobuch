use super::MultiSelection;

/// Which slots are currently selected. SlotSelection is always restricted to a single page.
pub struct SlotSelection {
    pub page: Option<usize>,
    sel: MultiSelection<usize>,
}

impl Default for SlotSelection {
    fn default() -> Self {
        Self {
            page: None,
            sel: MultiSelection::None,
        }
    }
}

impl SlotSelection {
    /// Select exactly one slot; replaces any previous selection.
    pub fn single(page: usize, slot: usize) -> Self {
        Self {
            page: Some(page),
            sel: MultiSelection::single(slot),
        }
    }

    /// Toggle `slot` on `page` (Ctrl+click). Different page resets to single.
    pub fn toggle(&mut self, page: usize, slot: usize) {
        if self.page == Some(page) {
            self.sel.toggle(slot);
            if self.sel.is_empty() {
                self.page = None;
            }
        } else {
            *self = Self::single(page, slot);
        }
    }

    /// Extend selection from anchor to `slot` (Shift+click). Different page resets to single.
    pub fn range_to(&mut self, page: usize, slot: usize) {
        if self.page == Some(page) {
            self.sel.range_to_numeric(slot);
        } else {
            *self = Self::single(page, slot);
        }
    }

    /// Clear selection (Escape / click on empty space).
    pub fn clear(&mut self) {
        self.page = None;
        self.sel.clear();
    }

    /// Select all slots on `page` (Ctrl+A). No-op when `slot_count == 0`.
    pub fn select_all_on(&mut self, page: usize, slot_count: usize) {
        if slot_count == 0 {
            self.page = None;
            self.sel.clear();
            return;
        }
        self.page = Some(page);
        self.sel = MultiSelection::from_range(0..=(slot_count - 1));
    }

    /// Returns `true` if `slot` on `page` is currently selected.
    pub fn is_selected(&self, page: usize, slot: usize) -> bool {
        self.page == Some(page) && self.sel.is_selected(&slot)
    }

    pub fn is_empty(&self) -> bool {
        self.sel.is_empty()
    }

    /// Returns selected slots on the active page.
    pub fn slots_on_active_page(&self) -> Vec<usize> {
        self.sel.items()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_replaces_previous() {
        let mut sel = SlotSelection::single(0, 2);
        assert!(sel.is_selected(0, 2));
        sel = SlotSelection::single(1, 5);
        assert!(sel.is_selected(1, 5));
        assert!(!sel.is_selected(0, 2));
    }

    #[test]
    fn toggle_adds_then_removes() {
        let mut sel = SlotSelection::single(0, 0);
        sel.toggle(0, 1);
        assert!(sel.is_selected(0, 0));
        assert!(sel.is_selected(0, 1));
        sel.toggle(0, 0);
        assert!(!sel.is_selected(0, 0));
        assert!(sel.is_selected(0, 1));
    }

    #[test]
    fn toggle_last_slot_gives_empty() {
        let mut sel = SlotSelection::single(0, 3);
        sel.toggle(0, 3);
        assert!(sel.is_empty());
        assert_eq!(sel.page, None);
    }

    #[test]
    fn range_fills_between_anchor_and_new_forward_and_backward() {
        let mut sel = SlotSelection::single(0, 2);
        sel.range_to(0, 5);
        for i in 2..=5 {
            assert!(sel.is_selected(0, i), "slot {i} should be selected");
        }
        sel.range_to(0, 0);
        for i in 0..=2 {
            assert!(
                sel.is_selected(0, i),
                "slot {i} should be selected after pull-back"
            );
        }
        assert!(!sel.is_selected(0, 3));
    }

    #[test]
    fn click_on_other_page_clears_and_starts_new() {
        let mut sel = SlotSelection::single(0, 3);
        sel.toggle(1, 7);
        assert!(!sel.is_selected(0, 3));
        assert!(sel.is_selected(1, 7));
    }

    #[test]
    fn select_all_picks_all_indices() {
        let mut sel = SlotSelection::default();
        sel.select_all_on(2, 4);
        for i in 0..4 {
            assert!(sel.is_selected(2, i));
        }
        assert!(!sel.is_selected(2, 4));
    }

    #[test]
    fn clear_resets_to_empty() {
        let mut sel = SlotSelection::single(0, 0);
        sel.clear();
        assert!(sel.is_empty());
        assert_eq!(sel.page, None);
    }
}
