//! Evolution dynamics for photo layout optimization.

pub(in crate::solver) mod crossover;
pub(in crate::solver) mod mutate;
mod selection;

use super::individual::LayoutIndividual;
use crate::models::{Canvas, FitnessWeights, Photo};
use crate::solver::ga_solver::{EvolutionDynamic, Individual};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Evolution dynamics for layout individuals.
pub struct LayoutEvolution {
    photos: Vec<Photo>,
    canvas: Canvas,
    weights: FitnessWeights,
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
    rng: StdRng,
}

impl LayoutEvolution {
    /// Creates evolution dynamics with given parameters.
    pub fn new(
        photos: Vec<Photo>,
        canvas: Canvas,
        weights: FitnessWeights,
        tournament_size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
        seed: u64,
    ) -> Self {
        Self {
            photos,
            canvas,
            weights,
            tournament_size,
            crossover_rate,
            mutation_rate,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl EvolutionDynamic<LayoutIndividual> for LayoutEvolution {
    fn select(&self, population: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        let count = population.len();
        select_parents(population, self.tournament_size, count, &mut self.rng.clone())
    }

    fn crossover(&self, parents: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        apply_crossover(
            parents,
            self.crossover_rate,
            &self.photos,
            &self.canvas,
            &self.weights,
            &mut self.rng.clone(),
        )
    }

    fn mutate(&self, individuals: &mut [LayoutIndividual]) {
        apply_mutation(
            individuals,
            self.mutation_rate,
            &self.photos,
            &self.canvas,
            &self.weights,
            &mut self.rng.clone(),
        );
    }
}

/// Selects parents using tournament selection.
fn select_parents<R: Rng>(
    population: &[LayoutIndividual],
    tournament_size: usize,
    count: usize,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    selection::tournament_select_with_fitness(
        population,
        tournament_size,
        count,
        rng,
        |ind| ind.fitness(),
    )
}

/// Applies crossover to parents with given rate.
fn apply_crossover<R: Rng>(
    parents: &[LayoutIndividual],
    crossover_rate: f64,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    let mut offspring = Vec::new();

    for chunk in parents.chunks(2) {
        if chunk.len() == 2 && rng.r#gen::<f64>() < crossover_rate {
            perform_crossover(chunk, photos, canvas, weights, rng, &mut offspring);
        } else {
            offspring.extend_from_slice(chunk);
        }
    }

    offspring
}

/// Performs crossover on a pair of parents.
fn perform_crossover<R: Rng>(
    pair: &[LayoutIndividual],
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    rng: &mut R,
    offspring: &mut Vec<LayoutIndividual>,
) {
    let tree_a = pair[0].tree();
    let tree_b = pair[1].tree();

    if let Some((child_a, child_b)) = crossover::crossover(tree_a, tree_b, rng) {
        offspring.push(LayoutIndividual::from_tree(child_a, photos, canvas, weights));
        offspring.push(LayoutIndividual::from_tree(child_b, photos, canvas, weights));
    } else {
        offspring.extend_from_slice(pair);
    }
}

/// Applies mutation to individuals with given rate.
fn apply_mutation<R: Rng>(
    individuals: &mut [LayoutIndividual],
    mutation_rate: f64,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    rng: &mut R,
) {
    for individual in individuals.iter_mut() {
        if rng.r#gen::<f64>() < mutation_rate {
            mutate_individual(individual, photos, canvas, weights, rng);
        }
    }
}

/// Mutates a single individual.
fn mutate_individual<R: Rng>(
    individual: &mut LayoutIndividual,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    rng: &mut R,
) {
    let mut tree = individual.tree().clone();
    mutate::mutate(&mut tree, rng);
    *individual = LayoutIndividual::from_tree(tree, photos, canvas, weights);
}
