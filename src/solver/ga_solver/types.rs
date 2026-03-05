//! Type definitions for the photobook layout GA.

use crate::models::{Canvas, FitnessWeights, PageLayout, Photo};
use super::super::page_layout_solver::tree::SlicingTree;
use super::traits::{Context, Genome, GeneticOperators, Individual, Phenotype};
use rand::Rng;

/// Parameters for generation operations.
///
/// Groups commonly-passed parameters into a single struct to reduce
/// function parameter count and improve maintainability.
#[derive(Clone, Copy)]
pub struct GenerationParams<'a> {
    pub photos: &'a [Photo],
    pub canvas: &'a Canvas,
    pub weights: &'a FitnessWeights,
    pub tournament_size: usize,
    pub crossover_rate: f64,
    pub mutation_rate: f64,
}

impl<'a> GenerationParams<'a> {
    pub fn new(
        photos: &'a [Photo],
        canvas: &'a Canvas,
        weights: &'a FitnessWeights,
        tournament_size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
    ) -> Self {
        Self {
            photos,
            canvas,
            weights,
            tournament_size,
            crossover_rate,
            mutation_rate,
        }
    }
}

/// Parameters for island operations.
///
/// Groups configuration and context for island-based GA runs.
pub struct IslandParams<'a> {
    pub photos: &'a [Photo],
    pub canvas: &'a Canvas,
    pub weights: &'a FitnessWeights,
    pub ga_config: &'a crate::models::GaConfig,
    pub island_config: &'a crate::models::IslandConfig,
}

impl<'a> IslandParams<'a> {
    pub fn new(
        photos: &'a [Photo],
        canvas: &'a Canvas,
        ga_config: &'a crate::models::GaConfig,
        island_config: &'a crate::models::IslandConfig,
    ) -> Self {
        Self {
            photos,
            canvas,
            weights: &ga_config.weights,
            ga_config,
            island_config,
        }
    }
    
    /// Creates GenerationParams from these island parameters.
    #[allow(dead_code)]
    pub fn to_generation_params(&self) -> GenerationParams<'a> {
        GenerationParams::new(
            self.photos,
            self.canvas,
            self.weights,
            self.ga_config.tournament_size,
            self.ga_config.crossover_rate,
            self.ga_config.mutation_rate,
        )
    }
}

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
        use super::super::page_layout_solver::solver::solve_layout;
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
