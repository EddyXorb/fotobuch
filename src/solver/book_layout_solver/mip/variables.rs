//! Variable management for the MIP solver.

use super::var_map::VarMap;
use good_lp::{ProblemVariables, Variable, variable};

/// All variables for the MIP problem.
#[derive(Debug)]
pub struct MipVariables {
    /// g_lj: Cumulative photos from group l on pages 1..j
    /// Domain: {0, ..., |G_l|}
    pub g: VarMap<2>,

    /// b_lj: Group l has photos on page j (binary)
    pub b: VarMap<2>,

    /// w_lj: Group l is fully on page j (binary)
    /// Only allocated for splittable groups (|G_l| >= g_min)
    pub w: VarMap<2>,

    /// a_j: Page j is active (binary)
    pub a: VarMap<1>,

    /// d_j: Deviation of page j size from target (continuous, >= 0)
    pub d: VarMap<1>,

    /// d_s: Deviation of page count from target (continuous, >= 0)
    pub d_s: Variable,
}

impl MipVariables {
    /// Creates all MIP variables.
    ///
    /// # Arguments
    ///
    /// * `problem` - The MIP problem to add variables to
    /// * `num_groups` - Number of groups (k)
    /// * `group_sizes` - Size of each group (|G_l|)
    /// * `b_max` - Maximum number of pages
    /// * `g_min` - Minimum photos for splitting a group
    #[allow(clippy::needless_range_loop)]
    pub fn new(
        problem: &mut ProblemVariables,
        num_groups: usize,
        group_sizes: &[usize],
        b_max: usize,
        g_min: usize,
    ) -> Self {
        let mut g = VarMap::new();
        let mut b = VarMap::new();
        let mut w = VarMap::new();
        let mut a = VarMap::new();
        let mut d = VarMap::new();

        // g_lj: cumulative photos from group l on pages 1..j
        // Domain: integer in [0, |G_l|]
        for l in 0..num_groups {
            for j in 0..=b_max {
                let var = problem.add(
                    variable()
                        .integer()
                        .min(0)
                        .max(group_sizes[l] as i32)
                        .name(format!("g_{}_{}", l, j)),
                );
                g.insert([l, j], var);
            }
        }

        // b_lj: group l has photos on page j
        // Domain: binary
        for l in 0..num_groups {
            for j in 1..=b_max {
                let var = problem.add(variable().binary().name(format!("b_{}_{}", l, j)));
                b.insert([l, j], var);
            }
        }

        // w_lj: group l is fully on page j
        // Only for splittable groups (|G_l| >= g_min)
        // Domain: binary
        for l in 0..num_groups {
            if group_sizes[l] >= g_min {
                for j in 1..=b_max {
                    let var = problem.add(variable().binary().name(format!("w_{}_{}", l, j)));
                    w.insert([l, j], var);
                }
            }
        }

        // a_j: page j is active
        // Domain: binary
        for j in 1..=b_max {
            let var = problem.add(variable().binary().name(format!("a_{}", j)));
            a.insert([j], var);
        }

        // d_j: deviation of page j size from target
        // Domain: continuous, >= 0
        for j in 1..=b_max {
            let var = problem.add(variable().min(0).name(format!("d_{}", j)));
            d.insert([j], var);
        }

        // d_s: deviation of page count from target
        // Domain: continuous, >= 0
        let d_s = problem.add(variable().min(0).name("d_s"));

        Self { g, b, w, a, d, d_s }
    }

    /// Returns n_lj (photos from group l on page j) as an expression.
    ///
    /// n_lj = g_lj - g_l(j-1)
    pub fn n_lj(&self, l: usize, j: usize) -> good_lp::Expression {
        use good_lp::Expression;

        let g_lj = self.g.get([l, j]);
        let g_lj_prev = if j > 0 {
            self.g.get([l, j - 1])
        } else {
            // g_l0 = 0 (boundary condition)
            return Expression::from(g_lj);
        };

        Expression::from(g_lj) - Expression::from(g_lj_prev)
    }

    pub fn len(&self) -> usize {
        self.g.len() + self.b.len() + self.w.len() + self.a.len() + self.d.len() + 1
    }
}
