//! Left/right parity of book pages (facing-pages layout).

/// Physical side of a book page when the book lies open.
///
/// Convention (as used by print services such as CEWE): the **first inner page
/// is a right-hand page**, the second a left-hand page, and so on.
///
/// Mapping to the 0-based layout index (`layout[i]`):
/// - with cover: `layout[0]` is the cover (no side), `layout[1]` is the first
///   inner page → odd layout indices are right-hand pages.
/// - without cover: `layout[0]` is the first inner page → even layout indices
///   are right-hand pages.
///
/// The *inside* edge is the edge at the spine (binding): left on right-hand
/// pages, right on left-hand pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSide {
    Left,
    Right,
}

impl PageSide {
    /// Side of the inner page with 0-based index `inner_idx` among inner pages
    /// (cover not counted). `0` → [`PageSide::Right`].
    pub fn for_inner_page(inner_idx: usize) -> Self {
        if inner_idx.is_multiple_of(2) {
            Self::Right
        } else {
            Self::Left
        }
    }

    /// Side of the page with 0-based layout index `page_idx`.
    ///
    /// Returns `None` for the cover (`has_cover && page_idx == 0`).
    pub fn for_page(page_idx: usize, has_cover: bool) -> Option<Self> {
        match (has_cover, page_idx) {
            (true, 0) => None,
            (true, i) => Some(Self::for_inner_page(i - 1)),
            (false, i) => Some(Self::for_inner_page(i)),
        }
    }

    /// `true` if the inside (spine) edge is the left page edge.
    pub fn is_inside_left(self) -> bool {
        self == Self::Right
    }

    /// Maps an `(inside, outside)` pair to `(left, right)` for this page side.
    pub fn to_left_right<T>(self, inside: T, outside: T) -> (T, T) {
        if self.is_inside_left() {
            (inside, outside)
        } else {
            (outside, inside)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_inner_page_is_right() {
        assert_eq!(PageSide::for_inner_page(0), PageSide::Right);
        assert_eq!(PageSide::for_inner_page(1), PageSide::Left);
        assert_eq!(PageSide::for_inner_page(2), PageSide::Right);
    }

    #[test]
    fn for_page_with_cover() {
        assert_eq!(PageSide::for_page(0, true), None);
        assert_eq!(PageSide::for_page(1, true), Some(PageSide::Right));
        assert_eq!(PageSide::for_page(2, true), Some(PageSide::Left));
        assert_eq!(PageSide::for_page(3, true), Some(PageSide::Right));
    }

    #[test]
    fn for_page_without_cover() {
        assert_eq!(PageSide::for_page(0, false), Some(PageSide::Right));
        assert_eq!(PageSide::for_page(1, false), Some(PageSide::Left));
        assert_eq!(PageSide::for_page(2, false), Some(PageSide::Right));
    }

    #[test]
    fn inside_edge() {
        assert!(PageSide::Right.is_inside_left());
        assert!(!PageSide::Left.is_inside_left());
    }

    #[test]
    fn to_left_right_maps_inside_to_spine() {
        assert_eq!(PageSide::Right.to_left_right(0.0, 3.0), (0.0, 3.0));
        assert_eq!(PageSide::Left.to_left_right(0.0, 3.0), (3.0, 0.0));
    }
}
