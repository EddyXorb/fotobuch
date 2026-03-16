//! Evolution dynamics for photo layout optimization.

pub(in crate::solver) mod crossover;
pub(in crate::solver) mod mutate;
mod selection;

use std::sync::atomic::AtomicU64;

use tracing::info;

use super::individual::LayoutIndividual;
use crate::dto_models::FitnessWeights;
use crate::solver::ga_solver::EvolutionDynamic;
use crate::solver::page_layout_solver::create_initial_population;
use crate::solver::prelude::*;

/// Context for evaluating slicing trees into individuals.
pub struct EvaluationContext<'a> {
    pub photos: &'a [Photo],
    pub canvas: &'a Canvas,
    pub weights: &'a FitnessWeights,
    pub enforce_order: bool,
    pub seed: AtomicU64,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(
        photos: &'a [Photo],
        canvas: &'a Canvas,
        weights: &'a FitnessWeights,
        enforce_order: bool,
        seed: u64,
    ) -> Self {
        Self {
            photos,
            canvas,
            weights,
            enforce_order,
            seed: AtomicU64::new(seed),
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
    fn are_identical(&self, left: &LayoutIndividual, right: &LayoutIndividual) -> bool {
        left.tree().has_same_internal_nodes_as(right.tree())
    }

    fn create(&self, nr: usize) -> Vec<LayoutIndividual> {
        if nr == 0 {
            return vec![];
        }
        info!("Create {} individuals!", nr);
        create_initial_population(&self.context, nr)
    }

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
            self.context.enforce_order,
        );
    }
}
