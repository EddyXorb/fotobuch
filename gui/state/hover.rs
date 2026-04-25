#[derive(Clone, Debug, PartialEq)]
pub enum HoveredTarget {
    /// Cursor over a central-panel page. `slot` is `Some` when a specific slot is hit.
    Page {
        page: usize,
        slot: Option<usize>,
    },
    NavPage(usize),
    PoolItem(String),
    /// `[+]`-placeholder between pages in the central panel.
    ///
    /// `at_position = i` means: new page lands **at index `i`**, the existing
    /// page at `i` shifts to `i+1`. Valid range: `0..=num_pages`.
    NewPageSlot {
        at_position: usize,
    },
}

impl HoveredTarget {
    /// Page index for both central-panel and nav-panel hovers.
    pub fn page_idx(&self) -> Option<usize> {
        match self {
            Self::Page { page, .. } | Self::NavPage(page) => Some(*page),
            Self::PoolItem(_) | Self::NewPageSlot { .. } => None,
        }
    }

    /// Central-panel page index only (nav pages excluded).
    pub fn central_page(&self) -> Option<usize> {
        match self {
            Self::Page { page, .. } => Some(*page),
            _ => None,
        }
    }

    /// `(page, slot_idx)` when hovering a specific slot.
    pub fn slot(&self) -> Option<(usize, usize)> {
        match self {
            Self::Page {
                page,
                slot: Some(slot),
            } => Some((*page, *slot)),
            _ => None,
        }
    }

    pub fn as_nav_page(&self) -> Option<usize> {
        match self {
            Self::NavPage(p) => Some(*p),
            _ => None,
        }
    }

    #[allow(unused)]
    pub fn as_pool_id(&self) -> Option<&str> {
        match self {
            Self::PoolItem(id) => Some(id),
            _ => None,
        }
    }

    pub fn new_page_at_position(&self) -> Option<usize> {
        match self {
            Self::NewPageSlot { at_position } => Some(*at_position),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_new_page_slot_returns_some_position() {
        let h = HoveredTarget::NewPageSlot { at_position: 3 };
        assert_eq!(h.new_page_at_position(), Some(3));
        assert_eq!(h.page_idx(), None);
        assert_eq!(h.slot(), None);
        assert_eq!(h.central_page(), None);
    }

    #[test]
    fn hover_new_page_slot_other_variants_return_none() {
        let h = HoveredTarget::Page {
            page: 1,
            slot: None,
        };
        assert_eq!(h.new_page_at_position(), None);
    }
}
