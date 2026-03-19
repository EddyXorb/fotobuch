//! Address types, command types, and error/result types for page commands.

// ── PagesExpr ─────────────────────────────────────────────────────────────────

/// A list of page numbers: `3`, `3,5`, or `3..5`.
#[derive(Debug, Clone, PartialEq)]
pub struct PagesExpr {
    pub pages: Vec<u32>,
}

impl PagesExpr {
    pub fn single(page: u32) -> Self {
        Self { pages: vec![page] }
    }

    pub fn from_list(pages: Vec<u32>) -> Self {
        Self { pages }
    }

    pub fn from_range(start: u32, end: u32) -> Self {
        Self {
            pages: (start..=end).collect(),
        }
    }
}

// ── SlotExpr ──────────────────────────────────────────────────────────────────

/// A set of slot indices: `2`, `2,7`, `2..5`, or `2..5,7`.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotExpr {
    pub slots: Vec<u32>,
}

impl SlotExpr {
    pub fn single(slot: u32) -> Self {
        Self { slots: vec![slot] }
    }

    pub fn from_list(slots: Vec<u32>) -> Self {
        Self { slots }
    }

    pub fn from_range(start: u32, end: u32) -> Self {
        Self {
            slots: (start..=end).collect(),
        }
    }
}

// ── Source / Destination types ────────────────────────────────────────────────

/// Source address for `page move` and `page swap`.
#[derive(Debug, Clone, PartialEq)]
pub enum Src {
    /// One or more full pages (all photos on those pages).
    Pages(PagesExpr),
    /// Specific slots on a single page.
    Slots { page: u32, slots: SlotExpr },
}

/// Destination for `page move ->`.
#[derive(Debug, Clone, PartialEq)]
pub enum DstMove {
    /// Existing page number.
    Page(u32),
    /// New page inserted directly after this page number.
    NewPageAfter(u32),
    /// Unplace the source photos (and delete the page if the source is whole pages).
    Unplace,
}

/// Destination for `page move <>` (swap).
#[derive(Debug, Clone, PartialEq)]
pub enum DstSwap {
    /// One or more full pages.
    Pages(PagesExpr),
    /// Specific slots on a single page.
    Slots { page: u32, slots: SlotExpr },
}

/// Parsed `page move` command — either a move or a swap.
#[derive(Debug, Clone, PartialEq)]
pub enum PageMoveCmd {
    Move { src: Src, dst: DstMove },
    Swap { left: Src, right: DstSwap },
}

// ── Error types ───────────────────────────────────────────────────────────────

/// Semantic validation errors (checked against the loaded project state).
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    PageNotFound(u32),
    SlotNotFound { page: u32, slot: u32 },
    SwapSamePage(u32),
    SwapCountMismatch { left: usize, right: usize },
    SwapRangesOverlap,
    CombineSinglePage(u32),
    SplitAtFirstSlot(u32),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PageNotFound(p) => write!(f, "page {p} does not exist"),
            Self::SlotNotFound { page, slot } => {
                write!(f, "slot {slot} does not exist on page {page}")
            }
            Self::SwapSamePage(p) => write!(f, "cannot swap page {p} with itself"),
            Self::SwapCountMismatch { left, right } => {
                write!(f, "swap requires equal page counts, got {left} vs {right}")
            }
            Self::SwapRangesOverlap => write!(f, "swap ranges must not overlap"),
            Self::CombineSinglePage(p) => {
                write!(f, "combine requires at least two pages, got only page {p}")
            }
            Self::SplitAtFirstSlot(p) => {
                write!(f, "cannot split at first slot (would leave page {p} empty)")
            }
        }
    }
}

/// Top-level error for page commands.
#[derive(Debug)]
pub enum PageMoveError {
    Validation(ValidationError),
    Other(anyhow::Error),
}

impl std::fmt::Display for PageMoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "{e}"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl From<anyhow::Error> for PageMoveError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

impl From<ValidationError> for PageMoveError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

// ── Result type ───────────────────────────────────────────────────────────────

/// Summary of what a page command changed.
#[derive(Debug)]
pub struct PageMoveResult {
    /// Pages whose photo list changed (need rebuild), 1-based.
    pub pages_modified: Vec<u32>,
    /// Pages that were newly inserted, 1-based.
    pub pages_inserted: Vec<u32>,
    /// Pages that were deleted, 1-based (original numbers before deletion).
    pub pages_deleted: Vec<u32>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pages_expr_from_range() {
        let pe = PagesExpr::from_range(3, 5);
        assert_eq!(pe.pages, vec![3, 4, 5]);
    }

    #[test]
    fn test_slot_expr_from_range() {
        let se = SlotExpr::from_range(2, 5);
        assert_eq!(se.slots, vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_validation_error_display() {
        assert_eq!(
            ValidationError::PageNotFound(5).to_string(),
            "page 5 does not exist"
        );
        assert_eq!(
            ValidationError::SlotNotFound { page: 3, slot: 7 }.to_string(),
            "slot 7 does not exist on page 3"
        );
        assert_eq!(
            ValidationError::SplitAtFirstSlot(2).to_string(),
            "cannot split at first slot (would leave page 2 empty)"
        );
    }
}
