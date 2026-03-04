//! Genetic algorithm main loop for photo layout optimization.

use crate::model::{Canvas, FitnessWeights, LayoutResult, Photo};
use super::tree::SlicingTree;
use rand::Rng;

/// Configuration for the genetic algorithm.
#[derive(Debug, Clone)]
pub struct GaConfig {
    /// Population size.
    pub population: usize,
    /// Maximum number of generations.
    pub generations: usize,
    /// Mutation probability.
    pub mutation_rate: f64,
    /// Crossover probability.
    pub crossover_rate: f64,
    /// Tournament selection size.
    pub tournament_size: usize,
    /// Elitism ratio (top % to keep unchanged).
    pub elitism_ratio: f64,
}

impl Default for GaConfig {
    fn default() -> Self {
        Self {
            population: 300,
            generations: 100,
            mutation_rate: 0.2,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.05,
        }
    }
}

/// Runs the genetic algorithm to find an optimal layout.
///
/// Returns the best tree, its layout, and its fitness cost.
pub fn run_ga<R: Rng>(
    _photos: &[Photo],
    canvas: &Canvas,
    _weights: &FitnessWeights,
    _config: &GaConfig,
    _rng: &mut R,
) -> (SlicingTree, LayoutResult, f64) {
    // TODO: Implement in Step 6
    // For now, return a dummy result
    let dummy_tree = SlicingTree::new(vec![super::tree::Node::Leaf {
        photo_idx: 0,
        parent: None,
    }]);
    let dummy_layout = LayoutResult::new(vec![], *canvas);
    (dummy_tree, dummy_layout, f64::INFINITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population, 300);
        assert_eq!(config.generations, 100);
    }
}
