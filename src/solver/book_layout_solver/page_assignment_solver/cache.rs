//! Dense grid cache for the page-assignment dynamic program.

use crate::solver::bellman_solver::{BellmanCache, StateValue};

use super::problem::State;

/// Flat `Vec` cache for the dense `(i, m)` state grid (`i ∈ [0, n]`,
/// `m ∈ [0, page_max]`). The state indexes directly into the vector, so lookups
/// avoid hashing entirely.
pub(super) struct GridCache {
    stride: usize,
    slots: Vec<Option<StateValue<usize>>>,
}

impl GridCache {
    pub(super) fn new(n: usize, page_max: usize) -> Self {
        let stride = n + 1;
        Self {
            stride,
            slots: vec![None; stride * (page_max + 1)],
        }
    }

    fn index(&self, (i, m): State) -> usize {
        m * self.stride + i
    }
}

impl BellmanCache<State, usize> for GridCache {
    fn get(&self, state: &State) -> Option<&StateValue<usize>> {
        self.slots[self.index(*state)].as_ref()
    }

    fn insert(&mut self, state: State, value: StateValue<usize>) {
        let idx = self.index(state);
        self.slots[idx] = Some(value);
    }
}
