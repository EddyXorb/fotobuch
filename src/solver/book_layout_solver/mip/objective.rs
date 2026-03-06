//! Objective function for the MIP solver.

use super::super::model::{GroupInfo, Params};
use super::variables::MipVariables;
use good_lp::Expression;

/// Builds the objective function: minimize w1*D_even + w2*D_split + w3*D_pages
pub fn build_objective(vars: &MipVariables, groups: &GroupInfo, params: &Params) -> Expression {
    let b_max = params.page_max;
    let num_groups = groups.num_groups();

    // D_even = sum_j d_j
    let d_even: Expression = (1..=b_max).map(|j| Expression::from(vars.d.get([j]))).sum();

    // D_split = sum_l (sum_j b_lj - 1)
    let d_split: Expression = (0..num_groups)
        .map(|l| {
            let sum_b: Expression = (1..=b_max).map(|j| Expression::from(vars.b.get([l, j]))).sum();
            sum_b - 1
        })
        .sum();

    // D_pages = d_s
    let d_pages: Expression = vars.d_s.into();

    // Combine with weights
    d_even * params.weight_even + d_split * params.weight_split + d_pages * params.weight_pages
}
