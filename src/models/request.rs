//! Solver request configuration bundling all parameters.

use super::{Canvas, GaConfig};
use std::path::PathBuf;

/// Complete solver request bundling all configuration parameters and I/O paths.
///
/// This struct encapsulates all parameters needed to run the photobook solver,
/// providing a clean API boundary between CLI parsing and the solver library.
#[derive(Debug, Clone)]
pub struct SolverRequest {
    /// Input directory containing photos
    pub input: PathBuf,

    /// Output file path (extension determines format: .json, .typ, .pdf)
    pub output: PathBuf,

    /// Canvas dimensions and spacing parameters
    pub canvas: Canvas,

    /// Genetic algorithm configuration (includes fitness weights, island config, and seed)
    pub ga_config: GaConfig,
}

impl SolverRequest {
    /// Create a new solver request with all required parameters.
    pub fn new(
        input: PathBuf,
        output: PathBuf,
        canvas: Canvas,
        ga_config: GaConfig,
    ) -> Self {
        Self {
            input,
            output,
            canvas,
            ga_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FitnessWeights, GaConfig, IslandConfig};

    #[test]
    fn test_solver_request_new() {
        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);

        let ga_config = GaConfig {
            population: 100,
            generations: 50,
            mutation_rate: 0.2,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.05,
            weights: FitnessWeights {
                w_size: 1.0,
                w_coverage: 0.15,
                w_barycenter: 0.5,
                w_order: 0.3,
            },
            timeout: None,
            island_config: IslandConfig {
                islands: 4,
                migration_interval: 5,
                migrants: 2,
            },
            seed: 42,
        };

        let request = SolverRequest::new(
            "input/".into(),
            "output.pdf".into(),
            canvas,
            ga_config,
        );

        assert_eq!(request.input.to_str().unwrap(), "input/");
        assert_eq!(request.output.to_str().unwrap(), "output.pdf");
        assert_eq!(request.ga_config.seed, 42);
        assert_eq!(request.canvas.width, 297.0);
    }
}
