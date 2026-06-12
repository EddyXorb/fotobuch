use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(crate) struct BellmanResult<Decision> {
    pub(crate) objective: f64,
    pub(crate) decisions: Vec<Decision>,
}

/// Cached value of a state: its optimal objective and the single decision that
/// achieves it (`None` for a terminal state). Storing only one decision per
/// state — instead of the whole tail path — keeps cache entries O(1); the full
/// path is reconstructed once at the end by walking the best decisions.
#[derive(Clone)]
struct StateValue<Decision> {
    objective: f64,
    best_decision: Option<Decision>,
}

/// Generic Bellman / dynamic-programming solver.
///
/// Solves `V(x) = min_{d in a(x)} { f(x, d) + V(t(x, d)) }` by backward
/// induction with memoization. States with an empty decision set are terminal;
/// their objective is given by `terminal_cost(x)` (`0.0` for a valid goal,
/// `f64::INFINITY` for an infeasible dead-end). `f64::INFINITY` propagates
/// through the recursion, so infeasible branches are never chosen unless every
/// branch is infeasible.
pub(crate) struct BellmanSolver<
    State,
    Decision,
    PossibleDecisions,
    TransitionFunction,
    CostFunction,
    TerminalCostFunction,
> where
    TransitionFunction: Fn(&State, &Decision) -> State,
    CostFunction: Fn(&State, &Decision) -> f64,
    PossibleDecisions: Fn(&State) -> Vec<Decision>,
    TerminalCostFunction: Fn(&State) -> f64,
    Decision: Clone,
    State: std::hash::Hash + Eq + Clone,
{
    x_0: State,
    t: TransitionFunction,
    f: CostFunction,
    a: PossibleDecisions,
    terminal_cost: TerminalCostFunction,

    value_cache: HashMap<State, StateValue<Decision>>,
    /// States on the current recursion path, used to detect cycles.
    visiting: HashSet<State>,
}

impl<State, Decision, PossibleDecisions, TransitionFunction, CostFunction, TerminalCostFunction>
    BellmanSolver<
        State,
        Decision,
        PossibleDecisions,
        TransitionFunction,
        CostFunction,
        TerminalCostFunction,
    >
where
    TransitionFunction: Fn(&State, &Decision) -> State,
    CostFunction: Fn(&State, &Decision) -> f64,
    PossibleDecisions: Fn(&State) -> Vec<Decision>,
    TerminalCostFunction: Fn(&State) -> f64,
    Decision: Clone,
    State: std::hash::Hash + Eq + Clone,
{
    pub(crate) fn new(
        x_0: State,
        t: TransitionFunction,
        f: CostFunction,
        a: PossibleDecisions,
        terminal_cost: TerminalCostFunction,
    ) -> Self {
        Self {
            x_0,
            t,
            f,
            a,
            terminal_cost,
            value_cache: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    pub(crate) fn solve(&mut self) -> BellmanResult<Decision> {
        let x_0 = self.x_0.clone();
        let objective = self.value_function(&x_0);
        let decisions = self.reconstruct_path(&x_0, objective);
        BellmanResult {
            objective,
            decisions,
        }
    }

    /// Compute and memoize `V(x)`. Returns only the objective; the optimal
    /// decision per state is recorded in `value_cache` for later reconstruction.
    fn value_function(&mut self, x: &State) -> f64 {
        // Cycle guard: revisiting a state on the current path means there is no
        // finite acyclic path through it here -> treat as infeasible. Not cached,
        // since this value is an artifact of the current path, not of `x` itself.
        if self.visiting.contains(x) {
            return f64::INFINITY;
        }

        if let Some(cached) = self.value_cache.get(x) {
            return cached.objective;
        }

        let possible_decisions = (self.a)(x);

        // Terminal state: no decisions left, value is the terminal cost.
        if possible_decisions.is_empty() {
            let objective = (self.terminal_cost)(x);
            self.value_cache.insert(
                x.clone(),
                StateValue {
                    objective,
                    best_decision: None,
                },
            );
            return objective;
        }

        self.visiting.insert(x.clone());

        let mut best_objective = f64::INFINITY;
        let mut best_decision: Option<Decision> = None;
        for d in &possible_decisions {
            let next_state = (self.t)(x, d);
            let candidate = (self.f)(x, d) + self.value_function(&next_state);
            if candidate < best_objective || best_decision.is_none() {
                best_objective = candidate;
                best_decision = Some(d.clone());
            }
        }

        self.visiting.remove(x);
        self.value_cache.insert(
            x.clone(),
            StateValue {
                objective: best_objective,
                best_decision,
            },
        );
        best_objective
    }

    /// Walk the cached best decisions forward from `x_0` to rebuild the optimal
    /// decision sequence. Returns an empty path for an infeasible (infinite)
    /// objective, where no valid plan exists.
    fn reconstruct_path(&self, x_0: &State, objective: f64) -> Vec<Decision> {
        if !objective.is_finite() {
            return Vec::new();
        }

        let mut decisions = Vec::new();
        let mut state = x_0.clone();
        while let Some(StateValue {
            best_decision: Some(d),
            ..
        }) = self.value_cache.get(&state)
        {
            let d = d.clone();
            state = (self.t)(&state, &d);
            decisions.push(d);
        }
        decisions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Partition `n` items into pieces; cost of a piece of size `p` is
    /// `(p - target)^2`. State is the number of remaining items.
    #[allow(clippy::type_complexity)] // generic solver type spelled out for the test helper
    fn partition_solver(
        n: usize,
        target: f64,
    ) -> BellmanSolver<
        usize,
        usize,
        impl Fn(&usize) -> Vec<usize>,
        impl Fn(&usize, &usize) -> usize,
        impl Fn(&usize, &usize) -> f64,
        impl Fn(&usize) -> f64,
    > {
        BellmanSolver::new(
            n,
            |x: &usize, d: &usize| x - d, // transition: remove a piece
            move |_x: &usize, d: &usize| {
                let diff = *d as f64 - target;
                diff * diff
            }, // cost
            |x: &usize| (1..=*x).collect::<Vec<_>>(), // pieces of size 1..=x
            |x: &usize| if *x == 0 { 0.0 } else { f64::INFINITY }, // terminal
        )
    }

    #[test]
    fn solves_trivial_terminal_root() {
        // Root is already terminal -> zero cost, no decisions.
        let mut solver = partition_solver(0, 3.0);
        let result = solver.solve();
        assert_eq!(result.objective, 0.0);
        assert!(result.decisions.is_empty());
    }

    #[test]
    fn finds_zero_cost_exact_partition() {
        // 6 items, target piece size 3 -> 3 + 3 with cost 0.
        let mut solver = partition_solver(6, 3.0);
        let result = solver.solve();
        assert_eq!(result.objective, 0.0);
        assert_eq!(result.decisions.iter().sum::<usize>(), 6);
        assert!(result.decisions.iter().all(|&p| p == 3));
    }

    #[test]
    fn finds_optimal_objective_with_remainder() {
        // 7 items, target 3 -> best is 3 + 4 (or 4 + 3), cost (1)^2 = 1.
        let mut solver = partition_solver(7, 3.0);
        let result = solver.solve();
        assert_eq!(result.objective, 1.0);
        assert_eq!(result.decisions.iter().sum::<usize>(), 7);
    }

    #[test]
    fn reconstructed_decisions_match_objective() {
        let mut solver = partition_solver(10, 3.0);
        let result = solver.solve();

        // Recompute the cost from the reconstructed decisions.
        let target = 3.0;
        let recomputed: f64 = result
            .decisions
            .iter()
            .map(|&p| {
                let diff = p as f64 - target;
                diff * diff
            })
            .sum();
        assert_eq!(recomputed, result.objective);
        assert_eq!(result.decisions.iter().sum::<usize>(), 10);
    }

    /// Partition with allowed piece sizes restricted to {2, 3}. A leftover of
    /// size 1 is an infeasible dead-end.
    #[allow(clippy::type_complexity)] // generic solver type spelled out for the test helper
    fn restricted_solver(
        n: usize,
    ) -> BellmanSolver<
        usize,
        usize,
        impl Fn(&usize) -> Vec<usize>,
        impl Fn(&usize, &usize) -> usize,
        impl Fn(&usize, &usize) -> f64,
        impl Fn(&usize) -> f64,
    > {
        BellmanSolver::new(
            n,
            |x: &usize, d: &usize| x - d,
            |_x: &usize, _d: &usize| 0.0,
            |x: &usize| {
                [2usize, 3]
                    .into_iter()
                    .filter(|&p| p <= *x)
                    .collect::<Vec<_>>()
            },
            |x: &usize| if *x == 0 { 0.0 } else { f64::INFINITY },
        )
    }

    #[test]
    fn infeasible_root_returns_infinity() {
        // n = 1 cannot be partitioned into pieces of size 2 or 3.
        let mut solver = restricted_solver(1);
        let result = solver.solve();
        assert!(result.objective.is_infinite());
    }

    #[test]
    fn avoids_dead_end_branch() {
        // n = 4 with sizes {2, 3}: choosing 3 leads to leftover 1 (infeasible),
        // so the solver must choose 2 + 2.
        let mut solver = restricted_solver(4);
        let result = solver.solve();
        assert!(result.objective.is_finite());
        assert_eq!(result.objective, 0.0);
        assert_eq!(result.decisions, vec![2, 2]);
    }

    #[test]
    fn larger_feasible_restricted_instance() {
        // n = 7 = 2 + 2 + 3 (in some order); all costs zero.
        let mut solver = restricted_solver(7);
        let result = solver.solve();
        assert_eq!(result.objective, 0.0);
        assert_eq!(result.decisions.iter().sum::<usize>(), 7);
        assert!(result.decisions.iter().all(|&p| p == 2 || p == 3));
    }

    #[test]
    fn cycle_is_detected_and_returns_infinity() {
        // Self-looping state with no terminal reachable: the cycle guard must
        // prevent infinite recursion and yield infinity.
        let mut solver = BellmanSolver::new(
            0usize,
            |x: &usize, _d: &()| *x,    // transition back to itself
            |_x: &usize, _d: &()| 1.0,  // positive cost
            |_x: &usize| vec![()],      // always one decision
            |_x: &usize| f64::INFINITY, // never a valid terminal
        );
        let result = solver.solve();
        assert!(result.objective.is_infinite());
    }

    #[test]
    fn memoization_yields_optimal_result() {
        // Overlapping subproblems: many ways to reach the same remaining count.
        // The memoized result must still be optimal.
        let mut solver = partition_solver(12, 4.0);
        let result = solver.solve();
        // 12 = 4 + 4 + 4 -> cost 0.
        assert_eq!(result.objective, 0.0);
        assert_eq!(result.decisions.iter().sum::<usize>(), 12);
        assert!(result.decisions.iter().all(|&p| p == 4));
    }
}
