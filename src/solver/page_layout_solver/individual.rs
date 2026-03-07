//! Domain-specific Individual implementation for photo layout.

use super::super::data_models::PageLayout;
use super::affine_solver::solve_layout;
use super::evolution::EvaluationContext;
use super::fitness::total_cost;
use super::tree::SlicingTree;
use crate::solver::ga_solver::Individual;

/// Layout individual that wraps a slicing tree with evaluated layout.
#[derive(Clone)]
pub struct LayoutIndividual {
    tree: SlicingTree,
    layout: PageLayout,
    fitness: f64,
}

impl LayoutIndividual {
    /// Creates a new individual from a slicing tree.
    ///
    /// Evaluates the tree to compute layout and fitness.
    pub fn from_tree(tree: SlicingTree, context: &EvaluationContext) -> Self {
        let layout = solve_layout(&tree, context.photos, context.canvas);
        let fitness = total_cost(&layout, context.photos, context.canvas, context.weights);
        Self {
            tree,
            layout,
            fitness,
        }
    }

    /// Returns a reference to the slicing tree.
    pub fn tree(&self) -> &SlicingTree {
        &self.tree
    }

    /// Returns a reference to the page layout.
    pub fn layout(&self) -> &PageLayout {
        &self.layout
    }
}

impl Individual for LayoutIndividual {
    type Genome = SlicingTree;
    type Phenotype = PageLayout;

    fn genome(&self) -> &Self::Genome {
        &self.tree
    }

    fn phenotype(&self) -> &Self::Phenotype {
        &self.layout
    }

    fn fitness(&self) -> f64 {
        self.fitness
    }
}
