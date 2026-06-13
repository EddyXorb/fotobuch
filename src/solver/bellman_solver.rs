use std::collections::HashMap;

#[derive(Clone)]
pub(crate) struct BellmanResult<Decision> {
    pub(crate) objective: f64,
    pub(crate) decisions: Vec<Decision>,
}

/// Cached value of a state: its optimal objective and the single decision that
/// achieves it (`None` for a terminal state). Storing only one decision per
/// state — instead of the whole tail path — keeps cache entries O(1); the full
/// path is reconstructed once at the end by walking the best decisions.
///
/// `in_progress` flags a state currently on the recursion stack and doubles as
/// the cycle guard, so no separate "visiting" set (and no extra hashing) is
/// needed.
#[derive(Clone)]
pub(crate) struct StateValue<Decision> {
    pub(crate) objective: f64,
    pub(crate) best_decision: Option<Decision>,
    pub(crate) in_progress: bool,
}

/// Memoization store for the [`BellmanSolver`], mapping each state to its cached
/// [`StateValue`]. Abstracting it lets callers pick the representation that fits
/// their state space: a `HashMap` for sparse/arbitrary states, or a flat `Vec`
/// indexed directly by state for a dense grid (no hashing).
pub(crate) trait BellmanCache<State, Decision> {
    fn get(&self, state: &State) -> Option<&StateValue<Decision>>;
    fn insert(&mut self, state: State, value: StateValue<Decision>);
}

/// Default `HashMap`-backed cache for arbitrary hashable states.
pub(crate) struct HashCache<State, Decision> {
    map: HashMap<State, StateValue<Decision>>,
}

impl<State, Decision> Default for HashCache<State, Decision> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<State, Decision> BellmanCache<State, Decision> for HashCache<State, Decision>
where
    State: std::hash::Hash + Eq,
{
    fn get(&self, state: &State) -> Option<&StateValue<Decision>> {
        self.map.get(state)
    }

    fn insert(&mut self, state: State, value: StateValue<Decision>) {
        self.map.insert(state, value);
    }
}

/// Generic Bellman / dynamic-programming solver.
///
/// Solves `V(x) = min_{d in a(x)} { f(x, d) + V(t(x, d)) }` by backward
/// induction with memoization. States with an empty decision set are terminal;
/// their objective is given by `terminal_cost(x)` (`0.0` for a valid goal,
/// `f64::INFINITY` for an infeasible dead-end). `f64::INFINITY` propagates
/// through the recursion, so infeasible branches are never chosen unless every
/// branch is infeasible.
///
/// The memoization store is pluggable via [`BellmanCache`]; it defaults to a
/// `HashMap`, but a dense state space can supply a flat `Vec` cache (via
/// [`BellmanSolver::with_cache`]) to avoid hashing entirely.
pub(crate) struct BellmanSolver<
    State,
    Decision,
    PossibleDecisions,
    TransitionFunction,
    CostFunction,
    TerminalCostFunction,
    Cache = HashCache<State, Decision>,
> where
    TransitionFunction: Fn(&State, &Decision) -> State,
    CostFunction: Fn(&State, &Decision) -> f64,
    PossibleDecisions: Fn(&State) -> Vec<Decision>,
    TerminalCostFunction: Fn(&State) -> f64,
    Cache: BellmanCache<State, Decision>,
    Decision: Clone,
    State: Clone,
{
    x_0: State,
    t: TransitionFunction,
    f: CostFunction,
    a: PossibleDecisions,
    terminal_cost: TerminalCostFunction,
    cache: Cache,
}

impl<State, Decision, PossibleDecisions, TransitionFunction, CostFunction, TerminalCostFunction>
    BellmanSolver<
        State,
        Decision,
        PossibleDecisions,
        TransitionFunction,
        CostFunction,
        TerminalCostFunction,
        HashCache<State, Decision>,
    >
where
    TransitionFunction: Fn(&State, &Decision) -> State,
    CostFunction: Fn(&State, &Decision) -> f64,
    PossibleDecisions: Fn(&State) -> Vec<Decision>,
    TerminalCostFunction: Fn(&State) -> f64,
    Decision: Clone,
    State: std::hash::Hash + Eq + Clone,
{
    /// Creates a solver backed by the default `HashMap` cache.
    #[allow(dead_code)] // ergonomic default-cache constructor, exercised by unit tests
    pub(crate) fn new(
        x_0: State,
        t: TransitionFunction,
        f: CostFunction,
        a: PossibleDecisions,
        terminal_cost: TerminalCostFunction,
    ) -> Self {
        Self::with_cache(x_0, t, f, a, terminal_cost, HashCache::default())
    }
}

impl<
    State,
    Decision,
    PossibleDecisions,
    TransitionFunction,
    CostFunction,
    TerminalCostFunction,
    Cache,
>
    BellmanSolver<
        State,
        Decision,
        PossibleDecisions,
        TransitionFunction,
        CostFunction,
        TerminalCostFunction,
        Cache,
    >
where
    TransitionFunction: Fn(&State, &Decision) -> State,
    CostFunction: Fn(&State, &Decision) -> f64,
    PossibleDecisions: Fn(&State) -> Vec<Decision>,
    TerminalCostFunction: Fn(&State) -> f64,
    Cache: BellmanCache<State, Decision>,
    Decision: Clone,
    State: Clone,
{
    /// Creates a solver backed by a caller-provided cache. Useful for dense
    /// state spaces that index a flat `Vec` instead of hashing.
    pub(crate) fn with_cache(
        x_0: State,
        t: TransitionFunction,
        f: CostFunction,
        a: PossibleDecisions,
        terminal_cost: TerminalCostFunction,
        cache: Cache,
    ) -> Self {
        Self {
            x_0,
            t,
            f,
            a,
            terminal_cost,
            cache,
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
    /// decision per state is recorded in the cache for later reconstruction.
    fn value_function(&mut self, x: &State) -> f64 {
        if let Some(cached) = self.cache.get(x) {
            // A state still on the recursion stack (`in_progress`) closes a
            // cycle: no finite acyclic path runs through it here, so it is
            // infeasible. This value is an artifact of the current path and is
            // not the final cached value, so it is not written back.
            return if cached.in_progress {
                f64::INFINITY
            } else {
                cached.objective
            };
        }

        let possible_decisions = (self.a)(x);

        // Terminal state: no decisions left, value is the terminal cost.
        if possible_decisions.is_empty() {
            let objective = (self.terminal_cost)(x);
            self.cache.insert(
                x.clone(),
                StateValue {
                    objective,
                    best_decision: None,
                    in_progress: false,
                },
            );
            return objective;
        }

        // Mark the state in progress before recursing (cycle guard).
        self.cache.insert(
            x.clone(),
            StateValue {
                objective: f64::INFINITY,
                best_decision: None,
                in_progress: true,
            },
        );

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

        self.cache.insert(
            x.clone(),
            StateValue {
                objective: best_objective,
                best_decision,
                in_progress: false,
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
        }) = self.cache.get(&state)
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
