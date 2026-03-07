//! Book layout containing multiple pages.

use super::layout::PageLayout;

/// Complete photobook layout containing all pages.
///
/// A book layout consists of one or more pages, each with its own photo placements.
/// Pages are ordered sequentially and will be exported in this order.
#[derive(Debug, Clone)]
pub struct BookLayout {
    /// All pages in the book, in order.
    pub pages: Vec<PageLayout>,
}

impl BookLayout {
    /// Creates a new book layout with the given pages.
    pub fn new(pages: Vec<PageLayout>) -> Self {
        Self { pages }
    }

    /// Creates a book layout with a single page.
    pub fn single_page(page: PageLayout) -> Self {
        Self { pages: vec![page] }
    }

    /// Returns the number of pages in the book.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns the total number of photos across all pages.
    pub fn total_photo_count(&self) -> usize {
        self.pages.iter().map(|p| p.placements.len()).sum()
    }

    /// Returns true if the book has no pages or all pages are empty.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() || self.pages.iter().all(|p| p.placements.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Canvas, PhotoPlacement};
    use super::*;

    #[test]
    fn test_book_layout_new() {
        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
        let page = PageLayout::new(vec![], canvas);
        let book = BookLayout::new(vec![page]);

        assert_eq!(book.page_count(), 1);
        assert_eq!(book.total_photo_count(), 0);
    }

    #[test]
    fn test_book_layout_single_page() {
        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
        let placements = vec![PhotoPlacement::new(0, 10.0, 10.0, 100.0, 100.0)];
        let page = PageLayout::new(placements, canvas);
        let book = BookLayout::single_page(page);

        assert_eq!(book.page_count(), 1);
        assert_eq!(book.total_photo_count(), 1);
        assert!(!book.is_empty());
    }

    #[test]
    fn test_book_layout_multiple_pages() {
        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);

        let page1 = PageLayout::new(
            vec![PhotoPlacement::new(0, 10.0, 10.0, 100.0, 100.0)],
            canvas,
        );
        let page2 = PageLayout::new(
            vec![
                PhotoPlacement::new(1, 10.0, 10.0, 50.0, 50.0),
                PhotoPlacement::new(2, 70.0, 10.0, 50.0, 50.0),
            ],
            canvas,
        );

        let book = BookLayout::new(vec![page1, page2]);

        assert_eq!(book.page_count(), 2);
        assert_eq!(book.total_photo_count(), 3);
        assert!(!book.is_empty());
    }

    #[test]
    fn test_book_layout_is_empty() {
        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);

        // Empty book
        let empty_book = BookLayout::new(vec![]);
        assert!(empty_book.is_empty());

        // Book with empty pages
        let page = PageLayout::new(vec![], canvas);
        let book_with_empty_pages = BookLayout::new(vec![page]);
        assert!(book_with_empty_pages.is_empty());

        // Book with photos
        let page_with_photos = PageLayout::new(
            vec![PhotoPlacement::new(0, 10.0, 10.0, 100.0, 100.0)],
            canvas,
        );
        let book_with_photos = BookLayout::new(vec![page_with_photos]);
        assert!(!book_with_photos.is_empty());
    }
}
