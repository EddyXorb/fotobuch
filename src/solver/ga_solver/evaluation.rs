//! Individual evaluation and creation.

use super::types::LayoutIndividual;
use super::super::page_layout_solver::tree::SlicingTree;
use super::super::page_layout_solver::layout_solver::solve_layout;
use super::super::page_layout_solver::fitness::total_cost;
use crate::models::{Canvas, FitnessWeights, Photo};

/// Creates an individual from a genome (tree).
pub fn create_from_genome(
    tree: SlicingTree,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
) -> LayoutIndividual {
    let layout = solve_layout(&tree, photos, canvas);
    let fitness = total_cost(&layout, photos, canvas, weights);
    LayoutIndividual { tree, layout, fitness }
}

/// Evaluates offspring and adds them to the population.
pub fn evaluate_offspring(
    child1_tree: SlicingTree,
    child2_tree: SlicingTree,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    population: &mut Vec<LayoutIndividual>,
    target_size: usize,
) {
    // Evaluate first child
    population.push(create_from_genome(child1_tree, photos, canvas, weights));
    
    // Evaluate second child if we haven't reached target size
    if population.len() < target_size {
        population.push(create_from_genome(child2_tree, photos, canvas, weights));
    }
}
