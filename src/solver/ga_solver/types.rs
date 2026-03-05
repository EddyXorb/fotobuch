//! Type definitions for the photobook layout GA.

use crate::models::{Canvas, FitnessWeights, PageLayout, Photo};
use super::super::page_layout_solver::tree::SlicingTree;
use super::traits::{Context, Genome, GeneticOperators, Individual, Phenotype};
use rand::Rng;

/// Context for photobook layout optimization.
#[derive(Clone)]
#[allow(dead_code)]
pub struct LayoutContext<'a> {
    pub photos: &'a [Photo],
    pub canvas: Canvas,
    pub weights: FitnessWeights,
}

impl Context for LayoutContext<'_> {}

/// Genome implementation for SlicingTree.
impl Genome for SlicingTree {
    fn random<R: Rng>(size: usize, rng: &mut R) -> Self {
        use super::super::page_layout_solver::tree::build::random_tree;
        random_tree(size, rng)
    }
}

/// Phenotype implementation for PageLayout.
impl Phenotype for PageLayout {
    type Genome = SlicingTree;
    type Context = LayoutContext<'static>;
    
    fn from_genome(genome: &Self::Genome, context: &Self::Context) -> Self {
        use super::super::page_layout_solver::layout_solver::solve_layout;
        solve_layout(genome, context.photos, &context.canvas)
    }
    
    fn fitness(&self, context: &Self::Context) -> f64 {
        use super::super::page_layout_solver::fitness::total_cost;
        total_cost(self, context.photos, &context.canvas, &context.weights)
    }
}

/// Genetic operators for SlicingTree.
#[allow(dead_code)]
pub struct TreeOperators;

impl GeneticOperators for TreeOperators {
    type Genome = SlicingTree;
    
    fn crossover<R: Rng>(
        parent1: &Self::Genome,
        parent2: &Self::Genome,
        rng: &mut R,
    ) -> Option<(Self::Genome, Self::Genome)> {
        use super::super::page_layout_solver::tree::crossover::crossover;
        crossover(parent1, parent2, rng)
    }
    
    fn mutate<R: Rng>(genome: &mut Self::Genome, rng: &mut R) {
        use super::super::page_layout_solver::tree::mutate::mutate;
        mutate(genome, rng);
    }
}

/// An individual in the population (genome + phenotype + fitness).
#[derive(Clone)]
pub struct LayoutIndividual {
    pub tree: SlicingTree,
    pub layout: PageLayout,
    pub fitness: f64,
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
    
    fn new(genome: Self::Genome, phenotype: Self::Phenotype, fitness: f64) -> Self {
        Self {
            tree: genome,
            layout: phenotype,
            fitness,
        }
    }
}
