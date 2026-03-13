//! Layout cache for avoiding redundant page layout solver calls.
//!
//! Stores the best known layout result for each page photo range.

use crate::solver::page_layout_solver::{CostBreakdown, GaResult};
use crate::solver::prelude::Photo;
use std::collections::{BTreeSet, HashMap};

/// Trait for types that can be cached by fitness score.
/// Lower fitness is better.
pub trait HasFitness {
    fn fitness(&self) -> f64;
}

impl HasFitness for GaResult {
    fn fitness(&self) -> f64 {
        self.fitness
    }
}

impl HasFitness for CostBreakdown {
    fn fitness(&self) -> f64 {
        self.total
    }
}

/// Cache key identifying a unique page photo assignment by photo IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    ids: BTreeSet<String>,
}

impl CacheKey {
    fn new(photos: &[Photo]) -> Self {
        let ids = photos.iter().map(|p| p.id.clone()).collect();
        Self { ids }
    }
}

/// Cache for page layout results.
///
/// Stores the best known result for each photo set to avoid redundant solver calls.
/// Only stores better results (monotonic improvement).
#[derive(Debug, Default)]
pub struct PhotoCombinationCache<C: HasFitness> {
    entries: HashMap<CacheKey, C>,
}

impl<C: HasFitness> PhotoCombinationCache<C> {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Returns the cached result for a photo slice, if available.
    pub fn get(&self, photos: &[Photo]) -> Option<&C> {
        let key = CacheKey::new(photos);
        self.entries.get(&key)
    }

    /// Inserts a result if it's better than any existing entry for this photo set.
    ///
    /// Returns `true` if the entry was inserted/updated, `false` if the existing
    /// entry was better or equal.
    ///
    /// "Better" means lower fitness value.
    pub fn insert_if_better(&mut self, photos: &[Photo], result: C) -> bool {
        let key = CacheKey::new(photos);

        match self.entries.get(&key) {
            Some(existing) => {
                if result.fitness() < existing.fitness() {
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
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the cache is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries from the cache.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::data_models::Canvas;
    use super::*;
    use crate::solver::{
        page_layout_solver::{CostBreakdown, GaResult},
        prelude::*,
    };

    fn make_photo(id: &str) -> Photo {
        Photo::new(id.to_string(), 1.5, 1.0, "group".to_string())
    }

    fn make_ga_result(fitness: f64) -> GaResult {
        let canvas = Canvas::new(297.0, 210.0, 5.0);
        let layout = SolverPageLayout::new(vec![], canvas);
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

    fn make_breakdown(fitness: f64) -> CostBreakdown {
        CostBreakdown {
            total: fitness,
            size: 0.0,
            coverage: fitness,
            barycenter: 0.0,
            order: 0.0,
        }
    }

    #[test]
    fn test_cache_new() {
        let cache = PhotoCombinationCache::<GaResult>::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();
        let photos = vec![make_photo("a"), make_photo("b")];

        assert!(cache.insert_if_better(&photos, make_ga_result(0.15)));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.get(&photos);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let cache = PhotoCombinationCache::<GaResult>::new();
        assert!(cache.get(&[make_photo("a")]).is_none());
    }

    #[test]
    fn test_cache_insert_better() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();
        let photos = vec![make_photo("a")];

        cache.insert_if_better(&photos, make_ga_result(0.2));

        let better = cache.insert_if_better(&photos, make_ga_result(0.15));
        assert!(better, "Should accept better result");
        assert_eq!(cache.get(&photos).unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_insert_worse() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();
        let photos = vec![make_photo("a")];

        cache.insert_if_better(&photos, make_ga_result(0.15));

        let worse = cache.insert_if_better(&photos, make_ga_result(0.2));
        assert!(!worse, "Should reject worse result");
        assert_eq!(cache.get(&photos).unwrap().fitness, 0.15);
    }

    #[test]
    fn test_cache_insert_equal() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();
        let photos = vec![make_photo("a")];

        cache.insert_if_better(&photos, make_ga_result(0.15));

        let equal = cache.insert_if_better(&photos, make_ga_result(0.15));
        assert!(!equal, "Should reject equal result");
    }

    #[test]
    fn test_cache_key_is_id_based_not_order() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();
        let p1 = make_photo("a");
        let p2 = make_photo("b");

        cache.insert_if_better(&[p1.clone(), p2.clone()], make_ga_result(0.1));

        // Same photos, different order → same cache entry
        let retrieved = cache.get(&[p2, p1]);
        assert!(retrieved.is_some(), "Order should not matter");
        assert_eq!(retrieved.unwrap().fitness, 0.1);
    }

    #[test]
    fn test_cache_different_photo_sets_are_independent() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();
        let p1 = make_photo("a");
        let p2 = make_photo("b");
        let p3 = make_photo("c");

        cache.insert_if_better(&[p1.clone(), p2.clone()], make_ga_result(0.1));
        cache.insert_if_better(&[p1.clone(), p3.clone()], make_ga_result(0.2));

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&[p1.clone(), p2]).unwrap().fitness, 0.1);
        assert_eq!(cache.get(&[p1, p3]).unwrap().fitness, 0.2);
    }

    #[test]
    fn test_cache_with_cost_breakdown() {
        let mut cache = PhotoCombinationCache::<CostBreakdown>::new();
        let photos = vec![make_photo("x")];

        cache.insert_if_better(&photos, make_breakdown(0.3));

        let result = cache.get(&photos).unwrap();
        assert_eq!(result.total, 0.3);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = PhotoCombinationCache::<GaResult>::new();

        cache.insert_if_better(&[make_photo("a")], make_ga_result(0.1));
        cache.insert_if_better(&[make_photo("b")], make_ga_result(0.2));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.get(&[make_photo("a")]).is_none());
    }
}
