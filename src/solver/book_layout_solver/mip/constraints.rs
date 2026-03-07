//! Constraint building for the MIP solver.

use super::super::model::{GroupInfo, Params};
use super::variables::MipVariables;
use good_lp::Expression;

/// Builds all constraints for the MIP problem.
/// Returns a vector of constraint expressions.
pub fn build_constraints(
    vars: &MipVariables,
    groups: &GroupInfo,
    params: &Params,
) -> Vec<good_lp::Constraint> {
    let mut constraints = Vec::new();
    let num_groups = groups.num_groups();
    let b_max = params.page_max;

    // Collect all constraints
    constraints.extend(build_boundary_conditions(vars, groups));
    constraints.extend(build_monotonicity(vars, num_groups, b_max));
    constraints.extend(build_page_activity(vars, params));
    constraints.extend(build_page_size(vars, num_groups, b_max, params));
    constraints.extend(build_b_linking(vars, groups, b_max));
    constraints.extend(build_sequential_ordering(vars, groups, b_max));
    constraints.extend(build_max_groups_per_page(vars, num_groups, b_max, params));
    constraints.extend(build_group_splitting(vars, groups, b_max, params));
    constraints.extend(build_evenness(vars, groups, b_max, params));
    constraints.extend(build_page_count_deviation(vars, b_max, params));

    constraints
}

/// Boundary conditions: g_l0 = 0, g_l(b_max) = |G_l|
fn build_boundary_conditions(vars: &MipVariables, groups: &GroupInfo) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let num_groups = groups.num_groups();
    let b_max = vars.g.iter().map(|(idx, _)| idx[1]).max().unwrap_or(0);

    for l in 0..num_groups {
        // g_l0 = 0
        let g_l0 = vars.g.get([l, 0]);
        constraints.push(constraint!(g_l0 == 0));

        // g_l(b_max) = |G_l|
        let g_l_bmax = vars.g.get([l, b_max]);
        let group_size = groups.group_size(l) as i32;
        constraints.push(constraint!(g_l_bmax == group_size));
    }
    constraints
}

/// Monotonicity: g_lj >= g_l(j-1) for all l, j
fn build_monotonicity(
    vars: &MipVariables,
    num_groups: usize,
    b_max: usize,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();

    for l in 0..num_groups {
        for j in 1..=b_max {
            let g_lj = vars.g.get([l, j]);
            let g_lj_prev = vars.g.get([l, j - 1]);
            constraints.push(constraint!(g_lj >= g_lj_prev));
        }
    }
    constraints
}

/// Page activity: a_j >= a_(j+1) and page count bounds
fn build_page_activity(vars: &MipVariables, params: &Params) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let b_max = params.page_max;

    // Active pages are contiguous at the beginning: a_j >= a_(j+1)
    for j in 1..b_max {
        let a_j = vars.a.get([j]);
        let a_j_next = vars.a.get([j + 1]);
        constraints.push(constraint!(a_j >= a_j_next));
    }

    // Page count bounds: b_min <= sum_j a_j <= b_max
    let sum_a: Expression = (1..=b_max).map(|j| Expression::from(vars.a.get([j]))).sum();
    constraints.push(constraint!(sum_a.clone() >= params.page_min as i32));
    constraints.push(constraint!(sum_a <= b_max as i32));

    constraints
}

/// Page size: p_min * a_j <= sum_l n_lj <= p_max * a_j
fn build_page_size(
    vars: &MipVariables,
    num_groups: usize,
    b_max: usize,
    params: &Params,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();

    for j in 1..=b_max {
        let a_j = vars.a.get([j]);
        let sum_n: Expression = (0..num_groups).map(|l| vars.n_lj(l, j)).sum();

        // Lower bound: sum_l n_lj >= p_min * a_j
        let lower = Expression::from(a_j) * params.photos_per_page_min as i32;
        constraints.push(constraint!(sum_n.clone() >= lower));

        // Upper bound: sum_l n_lj <= p_max * a_j
        let upper = Expression::from(a_j) * params.photos_per_page_max as i32;
        constraints.push(constraint!(sum_n <= upper));
    }
    constraints
}

/// Linking b_lj: n_lj >= b_lj and n_lj <= |G_l| * b_lj
fn build_b_linking(
    vars: &MipVariables,
    groups: &GroupInfo,
    b_max: usize,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let num_groups = groups.num_groups();

    for l in 0..num_groups {
        for j in 1..=b_max {
            let n_lj = vars.n_lj(l, j);
            let b_lj = vars.b.get([l, j]);
            let group_size = groups.group_size(l) as i32;

            // n_lj >= b_lj
            constraints.push(constraint!(n_lj.clone() >= b_lj));

            // n_lj <= |G_l| * b_lj
            let upper = Expression::from(b_lj) * group_size;
            constraints.push(constraint!(n_lj <= upper));
        }
    }
    constraints
}

/// Sequential ordering: g_(l-1,j) >= |G_(l-1)| * b_lj
fn build_sequential_ordering(
    vars: &MipVariables,
    groups: &GroupInfo,
    b_max: usize,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let num_groups = groups.num_groups();

    for l in 1..num_groups {
        for j in 1..=b_max {
            let g_prev_j = vars.g.get([l - 1, j]);
            let b_lj = vars.b.get([l, j]);
            let prev_group_size = groups.group_size(l - 1) as i32;

            let rhs = Expression::from(b_lj) * prev_group_size;
            constraints.push(constraint!(g_prev_j >= rhs));
        }
    }
    constraints
}

/// Max groups per page: sum_l b_lj <= g_max
fn build_max_groups_per_page(
    vars: &MipVariables,
    num_groups: usize,
    b_max: usize,
    params: &Params,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();

    for j in 1..=b_max {
        let sum_b: Expression = (0..num_groups)
            .map(|l| Expression::from(vars.b.get([l, j])))
            .sum();
        constraints.push(constraint!(sum_b <= params.group_max_per_page as i32));
    }
    constraints
}

/// Group splitting constraints: w_lj linking and g_min rule
fn build_group_splitting(
    vars: &MipVariables,
    groups: &GroupInfo,
    b_max: usize,
    params: &Params,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let num_groups = groups.num_groups();

    for l in 0..num_groups {
        let group_size = groups.group_size(l);
        let is_splittable = group_size >= params.group_min_photos;

        for j in 1..=b_max {
            let n_lj = vars.n_lj(l, j);
            let b_lj = vars.b.get([l, j]);

            if is_splittable {
                // Splittable group: link w_lj
                let w_lj = vars.w.get([l, j]);

                // n_lj >= |G_l| * w_lj
                let lower = Expression::from(w_lj) * group_size as i32;
                constraints.push(constraint!(n_lj.clone() >= lower));

                // n_lj <= |G_l| - 1 + w_lj
                let upper = Expression::from(w_lj) + (group_size as i32 - 1);
                constraints.push(constraint!(n_lj.clone() <= upper));

                // g_min rule: n_lj >= g_min * (b_lj - w_lj)
                let rhs = (Expression::from(b_lj) - Expression::from(w_lj))
                    * params.group_min_photos as i32;
                constraints.push(constraint!(n_lj >= rhs));
            } else {
                // Not splittable: n_lj = |G_l| * b_lj (group must be placed whole)
                let rhs = Expression::from(b_lj) * group_size as i32;
                constraints.push(constraint!(n_lj == rhs));
            }
        }
    }
    constraints
}

/// Evenness: d_j >= |sum_l n_lj - n_bar| with big-M relaxation for inactive pages
fn build_evenness(
    vars: &MipVariables,
    groups: &GroupInfo,
    b_max: usize,
    params: &Params,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let total_photos = groups.total_photos() as f64;
    let n_bar = total_photos / params.page_target as f64;

    // Big-M: max(n_bar, p_max)
    let big_m = n_bar.max(params.photos_per_page_max as f64);

    for j in 1..=b_max {
        let sum_n: Expression = (0..groups.num_groups()).map(|l| vars.n_lj(l, j)).sum();
        let d_j = vars.d.get([j]);
        let a_j = vars.a.get([j]);

        // d_j >= sum_n - n_bar - M*(1 - a_j)
        // = sum_n - n_bar + M*a_j - M
        let rhs1 = sum_n.clone() - n_bar + Expression::from(a_j) * big_m - big_m;
        constraints.push(constraint!(d_j >= rhs1));

        // d_j >= n_bar - sum_n - M*(1 - a_j)
        // = n_bar - sum_n + M*a_j - M
        let rhs2 = n_bar - sum_n + Expression::from(a_j) * big_m - big_m;
        constraints.push(constraint!(d_j >= rhs2));
    }
    constraints
}

/// Page count deviation: d_s >= |sum_j a_j - s|
fn build_page_count_deviation(
    vars: &MipVariables,
    b_max: usize,
    params: &Params,
) -> Vec<good_lp::Constraint> {
    use good_lp::constraint;
    let mut constraints = Vec::new();
    let sum_a: Expression = (1..=b_max).map(|j| Expression::from(vars.a.get([j]))).sum();
    let d_s = vars.d_s;
    let s = params.page_target as i32;

    // d_s >= sum_a - s
    constraints.push(constraint!(d_s >= sum_a.clone() - s));

    // d_s >= s - sum_a
    constraints.push(constraint!(d_s >= s - sum_a));

    constraints
}
