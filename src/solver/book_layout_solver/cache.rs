//! Layout cache for avoiding redundant page layout solver calls.
//!
//! Stores the best known layout result for each page photo range.

use crate::solver::page_layout_solver::GaResult;
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
/// Stores the best known GA result for each photo range to avoid redundant solver calls.
/// Only stores better results (monotonic improvement).
#[derive(Debug, Default)]
pub struct LayoutCache {
    entries: HashMap<CacheKey, GaResult>,
}

impl LayoutCache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Returns the cached result for a photo range, if available.
    pub fn get(&self, range: Range<usize>) -> Option<&GaResult> {
        let key = CacheKey::new(range);
        self.entries.get(&key)
    }

    /// Inserts a result if it's better than any existing entry for this range.
    ///
    /// Returns `true` if the entry was inserted/updated, `false` if the existing
    /// entry was better or equal.
    ///
    /// "Better" means lower fitness value.
    pub fn insert_if_better(&mut self, range: Range<usize>, result: GaResult) -> bool {
        let key = CacheKey::new(range);

        match self.entries.get(&key) {
            Some(existing) => {
                // Only update if new result is better (lower fitness)
                if result.fitness < existing.fitness {
                    self.entries.insert(key, result);
                    true
                } else {
                    false
                }
            }
            None => {
                self.entries.insert(key, result);
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
    use super::super::super::data_models::Canvas;
    use crate::solver::page_layout_solver::{CostBreakdown, GaResult};

    fn make_dummy_result(fitness: f64) -> GaResult {
        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
        let layout = crate::dto_models::PageLayout::new(vec![], canvas);
        let breakdown = CostBreakdown {
            total: fitness,
            size: 0.0,
            coverage: fitness,
            barycenter: 0.0,
            order: 0.0,
        };

        GaResult {
            layout,
            fitness,
            cost_breakdown: breakdown,
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
        let result = make_dummy_result(0.15);

        assert!(cache.insert_if_better(0..5, result));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(0..5);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let cache = LayoutCache::new();
        assert!(cache.get(0..5).is_none());
    }

    #[test]
    fn test_cache_insert_better() {
        let mut cache = LayoutCache::new();

        // Insert initial result
        cache.insert_if_better(0..5, make_dummy_result(0.2));

        // Try to insert better result (lower fitness)
        let better = cache.insert_if_better(0..5, make_dummy_result(0.15));
        assert!(better, "Should accept better result");
        
        let retrieved = cache.get(0..5);
        assert_eq!(retrieved.unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_insert_worse() {
        let mut cache = LayoutCache::new();

        // Insert initial result
        cache.insert_if_better(0..5, make_dummy_result(0.15));

        // Try to insert worse result (higher fitness)
        let worse = cache.insert_if_better(0..5, make_dummy_result(0.2));
        assert!(!worse, "Should reject worse result");
        
        // Existing result should remain
        let retrieved = cache.get(0..5);
        assert_eq!(retrieved.unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_insert_equal() {
        let mut cache = LayoutCache::new();

        // Insert initial result
        cache.insert_if_better(0..5, make_dummy_result(0.15));

        // Try to insert equal result
        let equal = cache.insert_if_better(0..5, make_dummy_result(0.15));
        assert!(!equal, "Should reject equal result");
        
        // Existing result should remain
        let retrieved = cache.get(0..5);
        assert_eq!(retrieved.unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_multiple_ranges() {
        let mut cache = LayoutCache::new();

        cache.insert_if_better(0..5, make_dummy_result(0.1));
        cache.insert_if_better(5..10, make_dummy_result(0.2));
        cache.insert_if_better(10..15, make_dummy_result(0.3));

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(0..5).unwrap().fitness, 0.1);
        assert_eq!(cache.get(5..10).unwrap().fitness, 0.2);
        assert_eq!(cache.get(10..15).unwrap().fitness, 0.3);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = LayoutCache::new();

        cache.insert_if_better(0..5, make_dummy_result(0.1));
        cache.insert_if_better(5..10, make_dummy_result(0.2));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.get(0..5).is_none());
    }

    #[test]
    fn test_cache_different_ranges_are_independent() {
        let mut cache = LayoutCache::new();

        cache.insert_if_better(0..5, make_dummy_result(0.1));
        cache.insert_if_better(0..6, make_dummy_result(0.2)); // Different range

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(0..5).unwrap().fitness, 0.1);
        assert_eq!(cache.get(0..6).unwrap().fitness, 0.2);
    }
}
