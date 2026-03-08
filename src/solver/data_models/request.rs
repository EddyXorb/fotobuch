//! Solver request configuration bundling all parameters.

use super::Canvas;
use crate::dto_models::GaConfig;

/// Complete solver request bundling all configuration parameters and I/O paths.
///
/// This struct encapsulates all parameters needed to run the photobook solver,
/// providing a clean API boundary between CLI parsing and the solver library.
#[derive(Debug, Clone)]
pub struct SolverRequest {
    /// Canvas dimensions and spacing parameters
    pub canvas: Canvas,

    /// Genetic algorithm configuration (includes fitness weights, island config, and seed)
    pub ga_config: GaConfig,
}

impl SolverRequest {
    /// Create a new solver request with all required parameters.
    pub fn new(canvas: Canvas, ga_config: GaConfig) -> Self {
        Self { canvas, ga_config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_request_new() {
        let canvas = Canvas::new(297.0, 210.0, 5.0);
        let ga_config = crate::dto_models::GaConfig::default();

        let request = SolverRequest::new(canvas, ga_config.clone());

        assert_eq!(request.ga_config.seed, ga_config.seed);
        assert_eq!(request.canvas.width, 297.0);
    }
}
