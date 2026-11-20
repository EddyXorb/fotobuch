//! Left/right parity of book pages (facing-pages layout).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Physical side of a book page when the book lies open.
///
/// The side of the **first inner page** is configurable (`book.first_page_side`);
/// from there the sides alternate. The default is [`PageSide::Right`], the
/// convention of left-bound books whose inside covers stay unprinted: the first
/// inner page faces the inside of the front cover. Print services with
/// facing-page layouts (e.g. CEWE) use this convention. Products that also print
/// the inside of the front cover start with a left-hand page instead.
///
/// Mapping to the 0-based layout index (`layout[i]`), with `first = Right`:
/// - with cover: `layout[0]` is the cover (no side), `layout[1]` is the first
///   inner page → odd layout indices are right-hand pages.
/// - without cover: `layout[0]` is the first inner page → even layout indices
///   are right-hand pages.
///
/// With `first = Left` every side is swapped.
///
/// The *inside* edge is the edge at the spine (binding): left on right-hand
/// pages, right on left-hand pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageSide {
    Left,
    #[default]
    Right,
}

impl PageSide {
    /// Side of the inner page with 0-based index `inner_idx` among inner pages
    /// (cover not counted). `0` → `first`.
    pub fn for_inner_page(inner_idx: usize, first: Self) -> Self {
        if inner_idx.is_multiple_of(2) {
            first
        } else {
            first.opposite()
        }
    }

    /// Side of the page with 0-based layout index `page_idx` when the first
    /// inner page is on side `first`.
    ///
    /// Returns `None` for the cover (`has_cover && page_idx == 0`).
    pub fn for_page(page_idx: usize, has_cover: bool, first: Self) -> Option<Self> {
        match (has_cover, page_idx) {
            (true, 0) => None,
            (true, i) => Some(Self::for_inner_page(i - 1, first)),
            (false, i) => Some(Self::for_inner_page(i, first)),
        }
    }

    /// The other side.
    pub fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
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

impl fmt::Display for PageSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Left => "left",
            Self::Right => "right",
        })
    }
}

impl FromStr for PageSide {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            other => Err(format!(
                "invalid page side '{other}', expected 'left' or 'right'"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_inner_page_is_right_by_default() {
        let first = PageSide::default();
        assert_eq!(PageSide::for_inner_page(0, first), PageSide::Right);
        assert_eq!(PageSide::for_inner_page(1, first), PageSide::Left);
        assert_eq!(PageSide::for_inner_page(2, first), PageSide::Right);
    }

    #[test]
    fn first_inner_page_left_swaps_all_sides() {
        let first = PageSide::Left;
        assert_eq!(PageSide::for_inner_page(0, first), PageSide::Left);
        assert_eq!(PageSide::for_inner_page(1, first), PageSide::Right);
        assert_eq!(PageSide::for_page(0, true, first), None);
        assert_eq!(PageSide::for_page(1, true, first), Some(PageSide::Left));
        assert_eq!(PageSide::for_page(0, false, first), Some(PageSide::Left));
        assert_eq!(PageSide::for_page(1, false, first), Some(PageSide::Right));
    }

    #[test]
    fn parse_and_serde() {
        assert_eq!("left".parse::<PageSide>(), Ok(PageSide::Left));
        assert_eq!("Right".parse::<PageSide>(), Ok(PageSide::Right));
        assert!("up".parse::<PageSide>().is_err());
        assert_eq!(
            serde_yaml::to_string(&PageSide::Left).unwrap().trim(),
            "left"
        );
        assert_eq!(
            serde_yaml::from_str::<PageSide>("right").unwrap(),
            PageSide::Right
        );
        assert_eq!(PageSide::Left.to_string(), "left");
    }

    #[test]
    fn for_page_with_cover() {
        assert_eq!(PageSide::for_page(0, true, PageSide::Right), None);
        assert_eq!(
            PageSide::for_page(1, true, PageSide::Right),
            Some(PageSide::Right)
        );
        assert_eq!(
            PageSide::for_page(2, true, PageSide::Right),
            Some(PageSide::Left)
        );
        assert_eq!(
            PageSide::for_page(3, true, PageSide::Right),
            Some(PageSide::Right)
        );
    }

    #[test]
    fn for_page_without_cover() {
        assert_eq!(
            PageSide::for_page(0, false, PageSide::Right),
            Some(PageSide::Right)
        );
        assert_eq!(
            PageSide::for_page(1, false, PageSide::Right),
            Some(PageSide::Left)
        );
        assert_eq!(
            PageSide::for_page(2, false, PageSide::Right),
            Some(PageSide::Right)
        );
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
