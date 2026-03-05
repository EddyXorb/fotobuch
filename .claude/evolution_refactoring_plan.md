# Evolution Files Refactoring Plan

## Current Problems

### 1. Excessive Parameter Passing ⚠️
- `apply_crossover()`: 6 parameters including `photos`, `canvas`, `weights`
- `perform_crossover()`: 6 parameters repeating the same context
- `mutate_individual()`: 5 parameters doing the same
- Functions pass `(&[Photo], &Canvas, &FitnessWeights)` repeatedly

### 2. Bloated LayoutEvolution Struct 🔴
- Stores `photos: Vec<Photo>`, `canvas: Canvas`, `weights: FitnessWeights`
- These are only needed for creating/evaluating individuals, not for evolution logic
- Violates single responsibility principle

### 3. Inefficient RNG Handling ⚠️
- `self.rng.clone()` called 3 times per generation
- RNG should be borrowed mutably, not cloned

### 4. Selection Module Ignores Individual Trait 🔴
- `tournament_select_with_fitness()` requires a closure parameter
- Individual trait already has `fitness()` method!
- Generic `tournament_select()` is unused and returns 0.0

### 5. Unnecessary Wrapper Functions ⚠️
- `select_parents()` just calls `selection::tournament_select_with_fitness()`
- `apply_crossover()`/`perform_crossover()` create two-layer indirection
- `apply_mutation()`/`mutate_individual()` do the same

### 6. Tight Coupling 🔴
- `LayoutEvolution` directly owns domain data
- Hard to test operators independently
- Difficult to reuse operators

## Proposed Solution

### 1. Introduce EvaluationContext Struct

```rust
/// Context for evaluating slicing trees into individuals
pub struct EvaluationContext<'a> {
    photos: &'a [Photo],
    canvas: &'a Canvas,
    weights: &'a FitnessWeights,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(photos: &'a [Photo], canvas: &'a Canvas, weights: &'a FitnessWeights) -> Self {
        Self { photos, canvas, weights }
    }
}
```

**Benefits:**
- Reduces parameter count from 3 to 1
- Clearer semantic meaning
- Lifetime-bound, no unnecessary clones

### 2. Refactor LayoutEvolution to be Stateless

**Before:**
```rust
pub struct LayoutEvolution {
    photos: Vec<Photo>,        // Owned - unnecessary
    canvas: Canvas,            // Owned - unnecessary
    weights: FitnessWeights,   // Owned - unnecessary
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
    rng: StdRng,              // Owned - should be borrowed
}
```

**After:**
```rust
pub struct LayoutEvolution<'a> {
    context: EvaluationContext<'a>,  // Borrowed context
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
}
```

**Changes:**
- Remove owned `photos`, `canvas`, `weights`
- Remove `rng: StdRng` (pass as parameter to methods)
- Keep only configuration (rates, sizes)
- Add lifetime parameter for context

### 3. Fix Selection to Use Individual Trait

**Before:**
```rust
pub fn tournament_select_with_fitness<I, R, F>(
    population: &[I],
    tournament_size: usize,
    count: usize,
    rng: &mut R,
    fitness_fn: F,  // ← Unnecessary closure!
) -> Vec<I>
where
    I: Clone,
    R: Rng,
    F: Fn(&I) -> f64,
{
    // Uses closure to get fitness
}
```

**After:**
```rust
pub fn tournament_select<I, R>(
    population: &[I],
    tournament_size: usize,
    count: usize,
    rng: &mut R,
) -> Vec<I>
where
    I: Individual,  // ← Use trait bound!
    R: Rng,
{
    // Use I::fitness() directly
    let mut selected = Vec::with_capacity(count);
    for _ in 0..count {
        let winner = run_tournament(population, tournament_size, rng);
        selected.push(winner.clone());
    }
    selected
}

fn run_tournament<'a, I, R>(
    population: &'a [I],
    tournament_size: usize,
    rng: &mut R,
) -> &'a I
where
    I: Individual,
    R: Rng,
{
    let actual_size = tournament_size.min(population.len());
    let mut best = &population[rng.gen_range(0..population.len())];
    let mut best_fitness = best.fitness();  // ← Direct trait method!

    for _ in 1..actual_size {
        let candidate = &population[rng.gen_range(0..population.len())];
        let candidate_fitness = candidate.fitness();
        
        if candidate_fitness < best_fitness {
            best = candidate;
            best_fitness = candidate_fitness;
        }
    }
    best
}
```

### 4. Move Crossover Helpers to crossover.rs

Move `apply_crossover()` and `perform_crossover()` to `evolution/crossover.rs` as:

```rust
// In evolution/crossover.rs

/// Applies crossover to parents with given rate.
pub(super) fn apply_crossover<R: Rng>(
    parents: &[LayoutIndividual],
    crossover_rate: f64,
    context: &EvaluationContext,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    let mut offspring = Vec::with_capacity(parents.len());

    for chunk in parents.chunks_exact(2) {
        if rng.r#gen::<f64>() < crossover_rate {
            crossover_pair(chunk, context, rng, &mut offspring);
        } else {
            offspring.extend_from_slice(chunk);
        }
    }
    
    // Handle odd parent
    if parents.len() % 2 == 1 {
        offspring.push(parents.last().unwrap().clone());
    }
    
    offspring
}

/// Performs crossover on a pair of parents.
fn crossover_pair<R: Rng>(
    pair: &[LayoutIndividual],
    context: &EvaluationContext,
    rng: &mut R,
    offspring: &mut Vec<LayoutIndividual>,
) {
    let tree_a = pair[0].tree();
    let tree_b = pair[1].tree();

    if let Some((child_a, child_b)) = crossover(tree_a, tree_b, rng) {
        offspring.push(LayoutIndividual::from_tree(child_a, context));
        offspring.push(LayoutIndividual::from_tree(child_b, context));
    } else {
        offspring.extend_from_slice(pair);
    }
}
```

### 5. Move Mutation Helpers to mutate.rs

Move `apply_mutation()` and `mutate_individual()` to `evolution/mutate.rs` as:

```rust
// In evolution/mutate.rs

/// Applies mutation to individuals with given rate.
pub(super) fn apply_mutation<R: Rng>(
    individuals: &mut [LayoutIndividual],
    mutation_rate: f64,
    context: &EvaluationContext,
    rng: &mut R,
) {
    for individual in individuals.iter_mut() {
        if rng.r#gen::<f64>() < mutation_rate {
            mutate_individual(individual, context, rng);
        }
    }
}

/// Mutates a single individual.
fn mutate_individual<R: Rng>(
    individual: &mut LayoutIndividual,
    context: &EvaluationContext,
    rng: &mut R,
) {
    let mut tree = individual.tree().clone();
    mutate(&mut tree, rng);
    *individual = LayoutIndividual::from_tree(tree, context);
}
```

### 6. Simplify EvolutionDynamic Implementation

**After moving helpers, evolution.rs becomes minimal:**

```rust
//! Evolution dynamics for photo layout optimization.

pub(in crate::solver) mod crossover;
pub(in crate::solver) mod mutate;
mod selection;

use super::individual::LayoutIndividual;
use crate::models::{Canvas, FitnessWeights, Photo};
use crate::solver::ga_solver::{EvolutionDynamic, Individual};
use rand::Rng;

/// Context for evaluating slicing trees into individuals.
pub struct EvaluationContext<'a> {
    pub photos: &'a [Photo],
    pub canvas: &'a Canvas,
    pub weights: &'a FitnessWeights,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(photos: &'a [Photo], canvas: &'a Canvas, weights: &'a FitnessWeights) -> Self {
        Self { photos, canvas, weights }
    }
}

/// Evolution dynamics for layout individuals.
pub struct LayoutEvolution<'a> {
    context: EvaluationContext<'a>,
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
}

impl<'a> LayoutEvolution<'a> {
    pub fn new(
        context: EvaluationContext<'a>,
        tournament_size: usize,
        crossover_rate: f64,
        mutation_rate: f64,
    ) -> Self {
        Self {
            context,
            tournament_size,
            crossover_rate,
            mutation_rate,
        }
    }
}

impl<'a> EvolutionDynamic<LayoutIndividual> for LayoutEvolution<'a> {
    fn select(&self, population: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        // Direct call to selection - no wrapper
        selection::tournament_select(
            population,
            self.tournament_size,
            population.len(),
            &mut rand::thread_rng(),
        )
    }

    fn crossover(&self, parents: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        // Delegate to crossover module
        crossover::apply_crossover(
            parents,
            self.crossover_rate,
            &self.context,
            &mut rand::thread_rng(),
        )
    }

    fn mutate(&self, individuals: &mut [LayoutIndividual]) {
        // Delegate to mutate module
        mutate::apply_mutation(
            individuals,
            self.mutation_rate,
            &self.context,
            &mut rand::thread_rng(),
        );
    }
}
```

### 7. Update LayoutIndividual Constructor

```rust
impl LayoutIndividual {
    pub fn from_tree(
        tree: SlicingTree,
        context: &EvaluationContext,
    ) -> Self {
        let layout = solve_layout(&tree, context.photos, context.canvas);
        let fitness = total_cost(&layout, context.photos, context.canvas, context.weights);
        Self { tree, layout, fitness }
    }
}
```

### 8. Update run_ga() Call Site

```rust
pub(super) fn run_ga(
    photos: &[crate::models::Photo],
    canvas: &crate::models::Canvas,
    ga_config: &crate::models::GaConfig,
    seed: u64,
) -> (tree::SlicingTree, crate::models::PageLayout, f64) {
    use crate::solver::ga_solver::{Config, GeneticAlgorithm, Individual};

    // Create evaluation context
    let context = evolution::EvaluationContext::new(photos, canvas, &ga_config.weights);

    // Create initial population
    let initial_pop = create_initial_population(&context, ga_config.population, seed);

    // Create GA configuration
    let config = Config { /* ... */ };

    // Create evolution dynamics (no seed needed)
    let evolution = LayoutEvolution::new(
        context,
        ga_config.tournament_size,
        ga_config.crossover_rate,
        ga_config.mutation_rate,
    );

    // Run GA
    let mut ga = GeneticAlgorithm::new(config, evolution);
    let best = ga.solve(initial_pop).expect("GA returned no solution");

    // Extract results
    (best.tree().clone(), best.layout().clone(), best.fitness())
}

fn create_initial_population(
    context: &evolution::EvaluationContext,
    population_size: usize,
    seed: u64,
) -> Vec<LayoutIndividual> {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(seed);

    (0..population_size)
        .map(|_| {
            let tree = tree::create::random_tree(context.photos.len(), &mut rng);
            LayoutIndividual::from_tree(tree, context)
        })
        .collect()
}
```

## Impact Summary

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| LayoutEvolution fields | 7 | 4 | -43% |
| Parameter count (crossover) | 6 | 2 | -67% |
| Parameter count (mutation) | 5 | 2 | -60% |
| Helper function layers | 2 | 0 | -100% |
| RNG clones per generation | 3 | 0 | -100% |
| Selection complexity | Closure wrapper | Direct trait | ✓ |
| evolution.rs LOC | ~170 | ~70 | -59% |
| Code duplication | High | Low | ✓ |

## Implementation Order

1. ✅ **Phase 1**: Create `EvaluationContext` struct in evolution.rs
2. ✅ **Phase 2**: Fix `selection.rs` to use `Individual` trait directly
3. ✅ **Phase 3**: Move crossover helpers to `evolution/crossover.rs`
4. ✅ **Phase 4**: Move mutation helpers to `evolution/mutate.rs`
5. ✅ **Phase 5**: Refactor `LayoutEvolution` struct (remove owned data, lifetime param)
6. ✅ **Phase 6**: Update `LayoutIndividual::from_tree()` signature
7. ✅ **Phase 7**: Update call sites in `run_ga()` and `create_initial_population()`
8. ✅ **Phase 8**: Run tests, verify all 119 tests pass

## Expected Benefits

✅ **Reduced Complexity**: 59% fewer lines in evolution.rs  
✅ **Better Testability**: Operators can be tested independently  
✅ **Clearer Semantics**: EvaluationContext makes dependencies explicit  
✅ **Performance**: Eliminate unnecessary clones and allocations  
✅ **Maintainability**: Single responsibility, loose coupling  
✅ **Type Safety**: Lifetimes prevent dangling references  
✅ **Modularity**: Each operator file is self-contained with its helpers

## File Structure After Refactoring

```
src/solver/page_layout_solver/
├── evolution.rs              (~70 lines)
│   ├── EvaluationContext struct
│   ├── LayoutEvolution struct
│   └── EvolutionDynamic impl (delegates only)
├── evolution/
│   ├── selection.rs          (~60 lines)
│   │   └── tournament_select (uses Individual trait)
│   ├── crossover.rs          (~420 lines)
│   │   ├── crossover (tree operation)
│   │   ├── apply_crossover (high-level)
│   │   └── crossover_pair (helper)
│   └── mutate.rs            (~230 lines)
│       ├── mutate (tree operation)
│       ├── apply_mutation (high-level)
│       └── mutate_individual (helper)
└── individual.rs            (~65 lines)
    └── LayoutIndividual with updated from_tree()
```

## Notes

- **RNG Handling**: We'll need to address deterministic RNG in EvolutionDynamic trait
  - Option 1: Add RNG parameter to trait methods
  - Option 2: Use thread_rng() for now (simpler, non-deterministic)
  - Option 3: Store RNG in LayoutEvolution (current approach but without cloning)
  
- **Lifetime Management**: EvaluationContext<'a> ensures all references remain valid
  
- **Testing**: Each module can now be tested independently with mock contexts

## Constraints Maintained

✅ All functions ≤30 lines  
✅ No mod.rs files (Rust 2018+ idiom)  
✅ Conventional commits for each phase  
✅ All tests must pass after each phase
