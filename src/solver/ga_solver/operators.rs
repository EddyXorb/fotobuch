//! Genetic operators application.

use super::super::page_layout_solver::tree::SlicingTree;
use super::super::page_layout_solver::tree::crossover::crossover;
use super::super::page_layout_solver::tree::mutate::mutate;
use rand::Rng;

/// Applies crossover to two parent trees with given probability.
///
/// Returns two offspring trees. If crossover fails or doesn't occur,
/// returns clones of the parents.
pub fn apply_crossover<R: Rng>(
    parent1: &SlicingTree,
    parent2: &SlicingTree,
    crossover_rate: f64,
    rng: &mut R,
) -> (SlicingTree, SlicingTree) {
    if rng.gen_range(0.0..1.0) < crossover_rate
        && let Some((child1, child2)) = crossover(parent1, parent2, rng) {
            return (child1, child2);
        }
    (parent1.clone(), parent2.clone())
}

/// Applies mutation to a tree with given probability.
pub fn apply_mutation<R: Rng>(
    tree: &mut SlicingTree,
    mutation_rate: f64,
    rng: &mut R,
) {
    if rng.gen_range(0.0..1.0) < mutation_rate {
        mutate(tree, rng);
    }
}
