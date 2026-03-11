use std::collections::HashMap;

use good_lp::Variable;

/// Generic N-dimensional variable map.
///
/// Uses HashMap to allow sparse variable allocation (e.g., w_lj only for splittable groups).
#[derive(Debug)]
pub struct VarMap<const N: usize> {
    vars: HashMap<[usize; N], Variable>,
}

impl<const N: usize> VarMap<N> {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn insert(&mut self, index: [usize; N], var: Variable) {
        self.vars.insert(index, var);
    }
    #[allow(dead_code)]
    pub fn get(&self, index: [usize; N]) -> Variable {
        self.vars[&index]
    }
    #[allow(dead_code)]
    pub fn contains(&self, index: [usize; N]) -> bool {
        self.vars.contains_key(&index)
    }
    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = (&[usize; N], &Variable)> {
        self.vars.iter()
    }
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.vars.len()
    }
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }
}

impl<const N: usize> Default for VarMap<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use good_lp::{ProblemVariables, variable};

    /// Helper function to create test variables.
    fn setup_test_variables() -> (ProblemVariables, Variable, Variable, Variable) {
        let mut problem = ProblemVariables::new();
        let var1 = problem.add(variable().binary().name("test_var_1"));
        let var2 = problem.add(variable().binary().name("test_var_2"));
        let var3 = problem.add(variable().binary().name("test_var_3"));
        (problem, var1, var2, var3)
    }

    #[test]
    fn test_new_creates_empty_varmap() {
        let map: VarMap<2> = VarMap::new();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_default_creates_empty_varmap() {
        let map: VarMap<2> = VarMap::default();
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_is_empty_on_new_map() {
        let map: VarMap<1> = VarMap::new();
        assert!(map.is_empty());
    }

    #[test]
    fn test_len_on_empty_map() {
        let map: VarMap<3> = VarMap::new();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_insert_and_get_1d() {
        let (_problem, var1, var2, var3) = setup_test_variables();
        let mut map: VarMap<1> = VarMap::new();

        map.insert([0], var1);
        map.insert([1], var2);
        map.insert([5], var3);

        assert_eq!(map.get([0]), var1);
        assert_eq!(map.get([1]), var2);
        assert_eq!(map.get([5]), var3);
    }

    #[test]
    fn test_insert_and_get_2d() {
        let (_problem, var1, var2, var3) = setup_test_variables();
        let mut map: VarMap<2> = VarMap::new();

        map.insert([0, 0], var1);
        map.insert([1, 2], var2);
        map.insert([5, 7], var3);

        assert_eq!(map.get([0, 0]), var1);
        assert_eq!(map.get([1, 2]), var2);
        assert_eq!(map.get([5, 7]), var3);
    }

    #[test]
    fn test_insert_and_get_3d() {
        let (_problem, var1, var2, var3) = setup_test_variables();
        let mut map: VarMap<3> = VarMap::new();

        map.insert([0, 0, 0], var1);
        map.insert([1, 2, 3], var2);
        map.insert([5, 7, 9], var3);

        assert_eq!(map.get([0, 0, 0]), var1);
        assert_eq!(map.get([1, 2, 3]), var2);
        assert_eq!(map.get([5, 7, 9]), var3);
    }

    #[test]
    fn test_contains_after_insert() {
        let (_problem, var1, var2, _var3) = setup_test_variables();
        let mut map: VarMap<2> = VarMap::new();

        map.insert([0, 1], var1);
        map.insert([2, 3], var2);

        assert!(map.contains([0, 1]));
        assert!(map.contains([2, 3]));
    }

    #[test]
    fn test_contains_returns_false_for_missing() {
        let (_problem, var1, _var2, _var3) = setup_test_variables();
        let mut map: VarMap<2> = VarMap::new();

        map.insert([0, 1], var1);

        assert!(!map.contains([0, 0]));
        assert!(!map.contains([1, 1]));
        assert!(!map.contains([2, 3]));
    }

    #[test]
    fn test_len_after_insertions() {
        let (_problem, var1, var2, var3) = setup_test_variables();
        let mut map: VarMap<2> = VarMap::new();

        assert_eq!(map.len(), 0);

        map.insert([0, 0], var1);
        assert_eq!(map.len(), 1);

        map.insert([1, 2], var2);
        assert_eq!(map.len(), 2);

        map.insert([5, 7], var3);
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn test_is_empty_after_insertions() {
        let (_problem, var1, _var2, _var3) = setup_test_variables();
        let mut map: VarMap<1> = VarMap::new();

        assert!(map.is_empty());

        map.insert([0], var1);
        assert!(!map.is_empty());
    }

    #[test]
    fn test_iter_returns_all_elements() {
        let (_problem, var1, var2, var3) = setup_test_variables();
        let mut map: VarMap<2> = VarMap::new();

        map.insert([0, 0], var1);
        map.insert([1, 2], var2);
        map.insert([5, 7], var3);

        let mut count = 0;
        let mut found_indices = Vec::new();

        for (idx, _var) in map.iter() {
            count += 1;
            found_indices.push(*idx);
        }

        assert_eq!(count, 3);
        assert!(found_indices.contains(&[0, 0]));
        assert!(found_indices.contains(&[1, 2]));
        assert!(found_indices.contains(&[5, 7]));
    }

    #[test]
    #[should_panic]
    fn test_get_panics_on_missing_key() {
        let map: VarMap<2> = VarMap::new();
        let _ = map.get([0, 0]); // Should panic
    }

    #[test]
    fn test_insert_overwrites_existing_key() {
        let (_problem, var1, var2, _var3) = setup_test_variables();
        let mut map: VarMap<1> = VarMap::new();

        map.insert([0], var1);
        assert_eq!(map.get([0]), var1);
        assert_eq!(map.len(), 1);

        map.insert([0], var2);
        assert_eq!(map.get([0]), var2);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_sparse_allocation() {
        let (_problem, var1, var2, _var3) = setup_test_variables();
        let mut map: VarMap<2> = VarMap::new();

        // Insert variables at sparse indices
        map.insert([0, 0], var1);
        map.insert([100, 200], var2);

        assert_eq!(map.len(), 2);
        assert!(map.contains([0, 0]));
        assert!(map.contains([100, 200]));
        assert!(!map.contains([50, 100]));
    }
}
