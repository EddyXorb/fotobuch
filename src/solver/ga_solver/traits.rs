//! Generic traits for genetic algorithm components.

use rand::Rng;

/// Represents a genome that can be evolved.
#[allow(dead_code)]
pub trait Genome: Clone + Send {
    /// Creates a random genome.
    fn random<R: Rng>(size: usize, rng: &mut R) -> Self;
}

/// Context needed to evaluate a genome into a phenotype.
#[allow(dead_code)]
pub trait Context: Clone + Send + Sync {}

/// Represents a phenotype derived from a genome.
#[allow(dead_code)]
pub trait Phenotype: Clone + Send {
    type Genome: Genome;
    type Context: Context;
    
    /// Creates a phenotype from a genome given a context.
    fn from_genome(genome: &Self::Genome, context: &Self::Context) -> Self;
    
    /// Evaluates the fitness of this phenotype (lower is better).
    fn fitness(&self, context: &Self::Context) -> f64;
}

/// Genetic operators for a genome type.
#[allow(dead_code)]
pub trait GeneticOperators: Sized {
    type Genome: Genome;
    
    /// Applies crossover between two parent genomes.
    /// Returns None if crossover fails or is not applicable.
    fn crossover<R: Rng>(
        parent1: &Self::Genome,
        parent2: &Self::Genome,
        rng: &mut R,
    ) -> Option<(Self::Genome, Self::Genome)>;
    
    /// Mutates a genome in place.
    fn mutate<R: Rng>(genome: &mut Self::Genome, rng: &mut R);
}

/// An individual in the population.
#[allow(dead_code)]
pub trait Individual: Clone + Send {
    type Genome: Genome;
    type Phenotype: Phenotype<Genome = Self::Genome>;
    
    /// Returns a reference to the genome.
    fn genome(&self) -> &Self::Genome;
    
    /// Returns a reference to the phenotype.
    fn phenotype(&self) -> &Self::Phenotype;
    
    /// Returns the fitness value.
    fn fitness(&self) -> f64;
    
    /// Creates a new individual from a genome and context.
    fn new(
        genome: Self::Genome,
        phenotype: Self::Phenotype,
        fitness: f64,
    ) -> Self;
}
