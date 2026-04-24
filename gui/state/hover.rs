#[derive(Clone, Debug, PartialEq)]
pub enum HoveredTarget {
    /// Cursor over a central-panel page. `slot` is `Some` when a specific slot is hit.
    Page {
        page: usize,
        slot: Option<usize>,
    },
    NavPage(usize),
    PoolItem(String),
}

impl HoveredTarget {
    /// Page index for both central-panel and nav-panel hovers.
    pub fn page_idx(&self) -> Option<usize> {
        match self {
            Self::Page { page, .. } | Self::NavPage(page) => Some(*page),
            Self::PoolItem(_) => None,
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
}
