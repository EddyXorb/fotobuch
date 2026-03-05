/// Weights for the fitness function components.
#[derive(Debug, Clone, Copy)]
pub struct FitnessWeights {
    /// Weight for size distribution cost C1.
    pub w_size: f64,

    /// Weight for canvas coverage cost C2.
    pub w_coverage: f64,

    /// Weight for barycenter centering cost C_bary.
    pub w_barycenter: f64,

    /// Weight for reading order cost C_order.
    pub w_order: f64,
}

impl FitnessWeights {
    /// Creates new fitness weights.
    pub fn new(w_size: f64, w_coverage: f64, w_barycenter: f64, w_order: f64) -> Self {
        assert!(w_size >= 0.0, "w_size must be non-negative");
        assert!(w_coverage >= 0.0, "w_coverage must be non-negative");
        assert!(w_barycenter >= 0.0, "w_barycenter must be non-negative");
        assert!(w_order >= 0.0, "w_order must be non-negative");

        Self {
            w_size,
            w_coverage,
            w_barycenter,
            w_order,
        }
    }
}

impl Default for FitnessWeights {
    /// Returns the default weights as specified in the algorithm document.
    fn default() -> Self {
        Self {
            w_size: 1.0,
            w_coverage: 0.15,
            w_barycenter: 0.5,
            w_order: 0.3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_weights() {
        let w = FitnessWeights::new(1.0, 0.15, 0.5, 0.3);
        assert_eq!(w.w_size, 1.0);
        assert_eq!(w.w_coverage, 0.15);
        assert_eq!(w.w_barycenter, 0.5);
        assert_eq!(w.w_order, 0.3);
    }

    #[test]
    #[should_panic(expected = "w_size must be non-negative")]
    fn test_new_weights_negative_size() {
        FitnessWeights::new(-1.0, 0.15, 0.5, 0.3);
    }

    #[test]
    fn test_default_weights() {
        let w = FitnessWeights::default();
        assert_eq!(w.w_size, 1.0);
        assert_eq!(w.w_coverage, 0.15);
        assert_eq!(w.w_barycenter, 0.5);
        assert_eq!(w.w_order, 0.3);
    }

    #[test]
    fn test_zero_weights() {
        let w = FitnessWeights::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(w.w_size, 0.0);
        assert_eq!(w.w_coverage, 0.0);
        assert_eq!(w.w_barycenter, 0.0);
        assert_eq!(w.w_order, 0.0);
    }
}
