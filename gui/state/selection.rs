use std::collections::BTreeSet;

/// Which slots are currently selected. Selektion ist immer auf eine Seite beschränkt.
pub enum Selection {
    None,
    OnPage {
        page: usize,
        /// Sorted set → deterministic iteration and tests.
        slots: BTreeSet<usize>,
        /// Pivot for Shift+click range extension.
        anchor: usize,
    },
}

impl Selection {
    /// Select exactly one slot; replaces any previous selection.
    pub fn single(page: usize, slot: usize) -> Self {
        Selection::OnPage {
            page,
            slots: BTreeSet::from([slot]),
            anchor: slot,
        }
    }

    /// Toggle `slot` on `page` (Ctrl+click).
    ///
    /// Clicking on a different page clears the old selection and starts fresh.
    pub fn toggle(&mut self, page: usize, slot: usize) {
        match self {
            Selection::OnPage {
                page: p,
                slots,
                anchor,
            } if *p == page => {
                if slots.contains(&slot) {
                    slots.remove(&slot);
                    if slots.is_empty() {
                        *self = Selection::None;
                    }
                } else {
                    slots.insert(slot);
                    *anchor = slot;
                }
            }
            _ => {
                *self = Selection::single(page, slot);
            }
        }
    }

    /// Extend selection from anchor to `slot` (Shift+click).
    ///
    /// Replaces the entire slot set with `min(anchor, slot)..=max(anchor, slot)`.
    /// Anchor stays fixed so repeated Shift+clicks pull from the same origin.
    /// Clicking on a different page starts a fresh single selection.
    pub fn range_to(&mut self, page: usize, slot: usize) {
        match self {
            Selection::OnPage {
                page: p,
                slots,
                anchor,
            } if *p == page => {
                let a = *anchor;
                let lo = a.min(slot);
                let hi = a.max(slot);
                *slots = (lo..=hi).collect();
            }
            _ => {
                *self = Selection::single(page, slot);
            }
        }
    }

    /// Clear selection (Escape / click on empty space).
    pub fn clear(&mut self) {
        *self = Selection::None;
    }

    /// Select all slots on `page` (Ctrl+A). No-op when `slot_count == 0`.
    pub fn select_all_on(&mut self, page: usize, slot_count: usize) {
        if slot_count == 0 {
            *self = Selection::None;
            return;
        }
        *self = Selection::OnPage {
            page,
            slots: (0..slot_count).collect(),
            anchor: 0,
        };
    }

    /// Returns `true` if `slot` on `page` is currently selected.
    pub fn is_selected(&self, page: usize, slot: usize) -> bool {
        match self {
            Selection::OnPage { page: p, slots, .. } => *p == page && slots.contains(&slot),
            Selection::None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_replaces_previous() {
        let mut sel = Selection::single(0, 2);
        sel = Selection::single(1, 5);
        assert!(sel.is_selected(1, 5));
        assert!(!sel.is_selected(0, 2));
    }

    #[test]
    fn toggle_adds_then_removes() {
        let mut sel = Selection::single(0, 0);
        sel.toggle(0, 1);
        assert!(sel.is_selected(0, 0));
        assert!(sel.is_selected(0, 1));
        sel.toggle(0, 0);
        assert!(!sel.is_selected(0, 0));
        assert!(sel.is_selected(0, 1));
    }

    #[test]
    fn toggle_last_slot_gives_none() {
        let mut sel = Selection::single(0, 3);
        sel.toggle(0, 3);
        assert!(matches!(sel, Selection::None));
    }

    #[test]
    fn range_fills_between_anchor_and_new_forward_and_backward() {
        let mut sel = Selection::single(0, 2);
        sel.range_to(0, 5);
        for i in 2..=5 {
            assert!(sel.is_selected(0, i), "slot {i} should be selected");
        }
        // Pull back: anchor stays at 2
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
        let mut sel = Selection::single(0, 3);
        sel.toggle(1, 7);
        assert!(!sel.is_selected(0, 3));
        assert!(sel.is_selected(1, 7));
    }

    #[test]
    fn select_all_picks_all_indices() {
        let mut sel = Selection::None;
        sel.select_all_on(2, 4);
        for i in 0..4 {
            assert!(sel.is_selected(2, i));
        }
        assert!(!sel.is_selected(2, 4));
    }

    #[test]
    fn clear_resets_to_none() {
        let mut sel = Selection::single(0, 0);
        sel.clear();
        assert!(matches!(sel, Selection::None));
    }
}
