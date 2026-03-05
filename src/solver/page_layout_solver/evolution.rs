//! Evolution dynamics for photo layout optimization.

pub(in crate::solver) mod crossover;
pub(in crate::solver) mod mutate;
mod selection;

use super::individual::LayoutIndividual;
use crate::models::{Canvas, FitnessWeights, Photo};
use crate::solver::ga_solver::EvolutionDynamic;

/// Context for evaluating slicing trees into individuals.
pub struct EvaluationContext<'a> {
    pub photos: &'a [Photo],
    pub canvas: &'a Canvas,
    pub weights: &'a FitnessWeights,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(photos: &'a [Photo], canvas: &'a Canvas, weights: &'a FitnessWeights) -> Self {
        Self {
            photos,
            canvas,
            weights,
        }
    }
}

/// Evolution dynamics for layout individuals.
pub struct LayoutEvolution<'a> {
    context: EvaluationContext<'a>,
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
}

impl<'a> LayoutEvolution<'a> {
    /// Creates evolution dynamics with given parameters.
    pub fn new(
        context: EvaluationContext<'a>,
        tournament_size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
    ) -> Self {
        Self {
            context,
            tournament_size,
            crossover_rate,
            mutation_rate,
        }
    }
}

impl<'a> EvolutionDynamic<LayoutIndividual> for LayoutEvolution<'a> {
    fn select(&self, population: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        selection::tournament_select(
            population,
            self.tournament_size,
            population.len(),
            &mut rand::thread_rng(),
        )
    }

    fn crossover(&self, parents: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        crossover::apply_crossover(
            parents,
            self.crossover_rate,
            &self.context,
            &mut rand::thread_rng(),
        )
    }

    fn mutate(&self, individuals: &mut [LayoutIndividual]) {
        mutate::apply_mutation(
            individuals,
            self.mutation_rate,
            &self.context,
            &mut rand::thread_rng(),
        );
    }
}
