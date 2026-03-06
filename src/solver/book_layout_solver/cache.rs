//! Layout cache for avoiding redundant page layout solver calls.
//!
//! Stores the best known layout result for each page photo range.

use super::cost::PageCost;
use std::collections::HashMap;
use std::ops::Range;

/// Cache key identifying a unique page photo assignment.
///
/// Since photos are ordered, the range uniquely identifies a page.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    /// Range of photo indices on this page.
    photo_range: Range<usize>,
}

impl CacheKey {
    fn new(photo_range: Range<usize>) -> Self {
        Self { photo_range }
    }
}

/// Cache for page layout results.
///
/// Stores the best known cost for each photo range to avoid redundant solver calls.
/// Only stores better results (monotonic improvement).
#[derive(Debug, Default)]
pub struct LayoutCache {
    entries: HashMap<CacheKey, PageCost>,
}

impl LayoutCache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Returns the cached cost for a photo range, if available.
    pub fn get(&self, range: Range<usize>) -> Option<&PageCost> {
        let key = CacheKey::new(range);
        self.entries.get(&key)
    }

    /// Inserts a cost if it's better than any existing entry for this range.
    ///
    /// Returns `true` if the entry was inserted/updated, `false` if the existing
    /// entry was better or equal.
    ///
    /// "Better" means lower coverage cost (primary criterion).
    pub fn insert_if_better(&mut self, range: Range<usize>, cost: PageCost) -> bool {
        let key = CacheKey::new(range);

        match self.entries.get(&key) {
            Some(existing) => {
                // Only update if new cost is better (lower coverage)
                if cost.coverage < existing.coverage {
                    self.entries.insert(key, cost);
                    true
                } else {
                    false
                }
            }
            None => {
                self.entries.insert(key, cost);
                true
            }
        }
    }

    /// Returns the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cost(coverage: f64) -> PageCost {
        PageCost {
            total: coverage,
            size: 0.0,
            coverage,
            barycenter: 0.0,
            order: 0.0,
        }
    }

    #[test]
    fn test_cache_new() {
        let cache = LayoutCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = LayoutCache::new();
        let cost = make_cost(0.15);

        assert!(cache.insert_if_better(0..5, cost.clone()));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(0..5);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().coverage, 0.15);
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let cache = LayoutCache::new();
        assert!(cache.get(0..5).is_none());
    }

    #[test]
    fn test_cache_insert_better() {
        let mut cache = LayoutCache::new();

        // Insert initial cost
        cache.insert_if_better(0..5, make_cost(0.2));

        // Try to insert better cost
        let better = cache.insert_if_better(0..5, make_cost(0.15));
        assert!(better, "Should accept better cost");
        
        let retrieved = cache.get(0..5);
        assert_eq!(retrieved.unwrap().coverage, 0.15);
    }

    #[test]
    fn test_cache_insert_worse() {
        let mut cache = LayoutCache::new();

        // Insert initial cost
        cache.insert_if_better(0..5, make_cost(0.15));

        // Try to insert worse cost
        let worse = cache.insert_if_better(0..5, make_cost(0.2));
        assert!(!worse, "Should reject worse cost");
        
        // Existing cost should remain
        let retrieved = cache.get(0..5);
        assert_eq!(retrieved.unwrap().coverage, 0.15);
    }

    #[test]
    fn test_cache_insert_equal() {
        let mut cache = LayoutCache::new();

        // Insert initial cost
        cache.insert_if_better(0..5, make_cost(0.15));

        // Try to insert equal cost
        let equal = cache.insert_if_better(0..5, make_cost(0.15));
        assert!(!equal, "Should reject equal cost");
        
        // Existing cost should remain
        let retrieved = cache.get(0..5);
        assert_eq!(retrieved.unwrap().coverage, 0.15);
    }

    #[test]
    fn test_cache_multiple_ranges() {
        let mut cache = LayoutCache::new();

        cache.insert_if_better(0..5, make_cost(0.1));
        cache.insert_if_better(5..10, make_cost(0.2));
        cache.insert_if_better(10..15, make_cost(0.3));

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(0..5).unwrap().coverage, 0.1);
        assert_eq!(cache.get(5..10).unwrap().coverage, 0.2);
        assert_eq!(cache.get(10..15).unwrap().coverage, 0.3);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = LayoutCache::new();

        cache.insert_if_better(0..5, make_cost(0.1));
        cache.insert_if_better(5..10, make_cost(0.2));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.get(0..5).is_none());
    }

    #[test]
    fn test_cache_different_ranges_are_independent() {
        let mut cache = LayoutCache::new();

        cache.insert_if_better(0..5, make_cost(0.1));
        cache.insert_if_better(0..6, make_cost(0.2)); // Different range

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(0..5).unwrap().coverage, 0.1);
        assert_eq!(cache.get(0..6).unwrap().coverage, 0.2);
    }
}
