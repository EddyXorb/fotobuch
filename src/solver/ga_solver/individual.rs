//! Individual trait for genetic algorithms.
//!
//! Defines the interface for individuals in a GA population.

/// Represents an individual in a genetic algorithm population.
///
/// An individual consists of:
/// - **Genome**: The genetic representation (e.g., a tree, vector, graph)
/// - **Phenotype**: The expressed form of the genome (e.g., a layout, solution)
/// - **Fitness**: A measure of solution quality (lower is better in this implementation)
pub trait Individual: Clone {
    /// The genetic representation type.
    type Genome;

    /// The expressed phenotype type.
    type Phenotype;
    #[allow(dead_code)]
    /// Returns a reference to the genome.
    fn genome(&self) -> &Self::Genome;
    #[allow(dead_code)]
    /// Returns a reference to the phenotype.
    fn phenotype(&self) -> &Self::Phenotype;

    /// Returns the fitness value (lower is better).
    fn fitness(&self) -> f64;
}
