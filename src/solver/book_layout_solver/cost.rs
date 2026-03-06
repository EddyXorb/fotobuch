//! Cost structures for evaluating page layouts.
//!
//! These types are used to compute and compare the quality of different page assignments.

use super::super::page_layout_solver::CostBreakdown;
use std::cmp::Ordering;

/// Cost of a single page layout.
///
/// Derived from `CostBreakdown` in the page layout solver.
/// Lower cost means better quality.
#[derive(Debug, Clone, PartialEq)]
pub struct PageCost {
    /// Total cost.
    pub total: f64,
    /// Size distribution cost (penalty for uneven photo sizes).
    pub size: f64,
    /// Coverage cost (penalty for unused space).
    pub coverage: f64,
    /// Barycenter cost (penalty for off-center placement).
    pub barycenter: f64,
    /// Reading order cost (penalty for non-sequential photo ordering).
    pub order: f64,
}

impl From<CostBreakdown> for PageCost {
    fn from(breakdown: CostBreakdown) -> Self {
        Self {
            total: breakdown.total,
            size: breakdown.size,
            coverage: breakdown.coverage,
            barycenter: breakdown.barycenter,
            order: breakdown.order,
        }
    }
}

/// Cost of an entire book assignment.
///
/// Used to compare different page assignments in the local search.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentCost {
    /// Costs for individual pages.
    pub page_costs: Vec<PageCost>,
    /// Worst coverage cost across all pages (primary comparison criterion).
    pub worst: f64,
    /// Average coverage cost across all pages (secondary tiebreaker).
    pub average: f64,
    /// Index of the page with worst coverage.
    pub worst_page: usize,
}

impl AssignmentCost {
    /// Creates a new `AssignmentCost` from individual page costs.
    ///
    /// Computes `worst`, `average`, and `worst_page` from the coverage values.
    pub fn from_page_costs(page_costs: Vec<PageCost>) -> Self {
        if page_costs.is_empty() {
            return Self {
                page_costs,
                worst: 0.0,
                average: 0.0,
                worst_page: 0,
            };
        }

        let mut worst = 0.0;
        let mut worst_page = 0;
        let mut sum = 0.0;

        for (i, cost) in page_costs.iter().enumerate() {
            sum += cost.coverage;
            if cost.coverage > worst {
                worst = cost.coverage;
                worst_page = i;
            }
        }

        let average = sum / page_costs.len() as f64;

        Self {
            page_costs,
            worst,
            average,
            worst_page,
        }
    }

    /// Returns the number of pages.
    pub fn num_pages(&self) -> usize {
        self.page_costs.len()
    }
}

impl Eq for AssignmentCost {}

impl PartialOrd for AssignmentCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AssignmentCost {
    /// Compares assignments by worst coverage (lower is better), then by average coverage.
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: worst coverage (lower is better)
        match self.worst.partial_cmp(&other.worst) {
            Some(Ordering::Equal) => {}
            Some(ord) => return ord,
            None => return Ordering::Equal, // Handle NaN conservatively
        }

        // Secondary: average coverage (lower is better)
        match self.average.partial_cmp(&other.average) {
            Some(ord) => ord,
            None => Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page_cost(coverage: f64) -> PageCost {
        PageCost {
            total: coverage,
            size: 0.0,
            coverage,
            barycenter: 0.0,
            order: 0.0,
        }
    }

    #[test]
    fn test_assignment_cost_from_page_costs() {
        let page_costs = vec![
            make_page_cost(0.1),
            make_page_cost(0.3),
            make_page_cost(0.2),
        ];

        let assignment_cost = AssignmentCost::from_page_costs(page_costs);

        assert_eq!(assignment_cost.worst, 0.3);
        assert_eq!(assignment_cost.worst_page, 1);
        assert!((assignment_cost.average - 0.2).abs() < 1e-9);
        assert_eq!(assignment_cost.num_pages(), 3);
    }

    #[test]
    fn test_assignment_cost_empty() {
        let assignment_cost = AssignmentCost::from_page_costs(vec![]);

        assert_eq!(assignment_cost.worst, 0.0);
        assert_eq!(assignment_cost.worst_page, 0);
        assert_eq!(assignment_cost.average, 0.0);
        assert_eq!(assignment_cost.num_pages(), 0);
    }

    #[test]
    fn test_assignment_cost_single_page() {
        let page_costs = vec![make_page_cost(0.15)];
        let assignment_cost = AssignmentCost::from_page_costs(page_costs);

        assert_eq!(assignment_cost.worst, 0.15);
        assert_eq!(assignment_cost.worst_page, 0);
        assert_eq!(assignment_cost.average, 0.15);
        assert_eq!(assignment_cost.num_pages(), 1);
    }

    #[test]
    fn test_assignment_cost_ordering_worst() {
        let cost1 = AssignmentCost {
            page_costs: vec![],
            worst: 0.2,
            average: 0.15,
            worst_page: 0,
        };

        let cost2 = AssignmentCost {
            page_costs: vec![],
            worst: 0.3,
            average: 0.1, // Better average but worse worst
            worst_page: 0,
        };

        assert!(cost1 < cost2); // cost1 is better (lower worst)
    }

    #[test]
    fn test_assignment_cost_ordering_average_tiebreaker() {
        let cost1 = AssignmentCost {
            page_costs: vec![],
            worst: 0.2,
            average: 0.15,
            worst_page: 0,
        };

        let cost2 = AssignmentCost {
            page_costs: vec![],
            worst: 0.2, // Same worst
            average: 0.18,
            worst_page: 0,
        };

        assert!(cost1 < cost2); // cost1 is better (lower average)
    }

    #[test]
    fn test_assignment_cost_ordering_equal() {
        let cost1 = AssignmentCost {
            page_costs: vec![],
            worst: 0.2,
            average: 0.15,
            worst_page: 0,
        };

        let cost2 = AssignmentCost {
            page_costs: vec![],
            worst: 0.2,
            average: 0.15,
            worst_page: 1, // Different page, but same costs
        };

        // Ordering-wise they are equal (same worst and average)
        assert_eq!(cost1.cmp(&cost2), Ordering::Equal);
        // But structurally they differ (different worst_page)
        assert_ne!(cost1, cost2);
    }

    #[test]
    fn test_page_cost_from_cost_breakdown() {
        let breakdown = CostBreakdown {
            total: 1.0,
            size: 0.5,
            coverage: 0.3,
            barycenter: 0.1,
            order: 0.1,
        };

        let page_cost: PageCost = breakdown.into();

        assert_eq!(page_cost.total, 1.0);
        assert_eq!(page_cost.size, 0.5);
        assert_eq!(page_cost.coverage, 0.3);
        assert_eq!(page_cost.barycenter, 0.1);
        assert_eq!(page_cost.order, 0.1);
    }
}
