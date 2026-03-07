# Page Layout Solver Refactoring Plan

## Goal
Refactor the page_layout_solver to use the new generic GA solver infrastructure from `ga_solver/solver.rs`, creating a clear separation between the generic GA framework and the domain-specific photo layout logic.

## Current State Analysis

### Current Structure
```
src/solver/
├── ga_solver/
│   ├── solver.rs          # NEW: Generic GA framework (trait-based)
│   ├── evaluation.rs
│   ├── evolution.rs
│   ├── generation.rs
│   ├── island.rs
│   ├── operators.rs
│   ├── population.rs
│   ├── selection.rs
│   ├── traits.rs
│   └── types.rs
├── page_layout_solver/
│   ├── fitness.rs         # Domain: fitness calculation
│   ├── ga.rs             # Mixed: old GA + domain logic
│   ├── solver.rs         # Domain: affine layout solver
│   └── tree/
│       ├── build.rs
│       ├── crossover.rs
│       ├── mutate.rs
│       ├── operators.rs
│       └── validate.rs
└── book_layout_solver.rs  # Orchestration layer
```

### Problems
1. **ga_solver** contains domain-specific photo layout code mixed with generic GA
2. **page_layout_solver/ga.rs** duplicates generic GA logic
3. No clear trait implementations connecting generic GA to photo layout domain
4. Dependencies are not clean - GA depends on canvas, photos, etc.

## Target State

### New Structure (No mod.rs files, clear separation)
```
src/solver/
├── ga_solver.rs           # Module definition + exports
├── ga_solver/
│   ├── solver.rs          # GeneticAlgorithm<I, E> (from solver.rs)
│   ├── evolution.rs       # EvolutionDynamic trait + World/Island
│   ├── individual.rs      # Individual trait
│   └── config.rs          # Config struct
│
├── page_layout_solver.rs  # Module definition + exports
├── page_layout_solver/
│   ├── solver.rs          # HIGH-LEVEL: run_ga() orchestration only
│   ├── individual.rs      # Impl Individual trait for LayoutIndividual
│   ├── tree.rs            # SlicingTree data structure + submodule declarations
│   ├── fitness.rs         # FITNESS: total_cost(), size_penalty(), etc.
│   ├── affine_solver.rs   # LAYOUT: solve_layout() - affine layout solver
│   ├── evolution.rs       # EvolutionDynamic impl (GA orchestrator)
│   ├── tree/              # Tree data structure concerns:
│   │   ├── create.rs      # Random tree generation (renamed from build.rs)
│   │   ├── validate.rs    # Tree validation
│   │   ├── visualize.rs   # SVG visualization for debugging
│   │   └── proptests.rs   # Property-based tests for tree operations
│   └── evolution/         # GA operators (operate ON trees):
│       ├── selection.rs   # Tournament selection (operates on LayoutIndividual)
│       ├── crossover.rs   # Tree crossover operator
│       └── mutation.rs    # Tree mutation operator
│
└── book_layout_solver.rs  # Orchestration: multi-page logic
```

### Key Principles
1. **No mod.rs**: Use parent module file (e.g., `ga_solver.rs` for `ga_solver/` folder)
2. **Separation of Concerns**:
   - `tree/` = Pure data structure concerns (create, validate, visualize, tests)
   - `evolution/` = GA operators that operate ON trees (selection, crossover, mutation)
   - Tree module knows nothing about evolution; evolution imports and operates on trees
3. **Module Organization**:
   - `tree.rs` = SlicingTree data structure definition + submodule declarations
   - `evolution.rs` = EvolutionDynamic trait implementation (orchestrator)
4. **Property Tests Location**: `tree/proptests.rs` stays with tree module (tests tree invariants)
5. **One Responsibility per File**: Each operation gets its own file
6. **Naming Conventions**: `create.rs` (not build.rs), `mutation.rs` (not mutate.rs)
7. **Function Length Limit**: **All functions must stay below 30 lines of code**
   - Split large functions into smaller helper functions
   - Extract complex logic into well-named private functions
   - Applies to all code touched during refactoring

### Module Responsibilities

**tree/ - Data Structure Concerns:**
- `tree.rs`: SlicingTree, Node, Cut type definitions
- `create.rs`: Random tree construction
- `validate.rs`: Tree validation (structural invariants)
- `visualize.rs`: SVG visualization for debugging
- `proptests.rs`: Property-based tests for tree operations

**evolution/ - GA Operators (Operate ON Trees):**
- `selection.rs`: Tournament selection (operates on LayoutIndividual population)
- `crossover.rs`: Tree crossover operator (takes two trees, returns two offspring)
- `mutation.rs`: Tree mutation operator (mutates a tree in-place)

**Why This Separation?**
- **Tree doesn't know about evolution**: The tree module is a pure data structure with utilities
- **Evolution operates on trees**: GA operators import SlicingTree and manipulate it
- **Clear boundaries**: Tree construction/validation vs. genetic manipulation
- **Proptests location**: Tests tree invariants (create, validate), but also tests that evolution operators preserve those invariants - stays in tree/ since it tests tree properties

## Refactoring Steps

### Phase 1: Prepare Generic GA Module (No Domain Dependencies)

**Step 1.1: Create ga_solver.rs module file**

Create `src/solver/ga_solver.rs`:
```rust
//! Generic genetic algorithm framework
//!
//! This module provides a trait-based GA implementation that is completely
//! domain-agnostic. It can be used for any optimization problem by implementing
//! the Individual and EvolutionDynamic traits.
//!
//! # Architecture
//! - `solver`: Main GeneticAlgorithm struct
//! - `individual`: Individual trait definition
//! - `evolution`: EvolutionDynamic trait + World/Island infrastructure
//! - `config`: Configuration struct

mod solver;
mod config;
mod evolution;
mod individual;

pub use solver::GeneticAlgorithm;
pub use config::Config;
pub use evolution::{EvolutionDynamic, Island, World};
pub use individual::Individual;
```

**Step 1.2: Split solver.rs into modules**

Extract from `ga_solver/solver.rs`:
- Keep `struct GeneticAlgorithm` + impl → stays in `ga_solver/solver.rs`
- Move `trait Individual` → `ga_solver/individual.rs`
- Move `trait EvolutionDynamic`, `World`, `Island` → `ga_solver/evolution.rs`
- Move `struct Config` → `ga_solver/config.rs`

**Step 1.3: Update imports in ga_solver/solver.rs**
```rust
use super::config::Config;
use super::evolution::{EvolutionDynamic, Island, World};
use super::individual::Individual;
```

**Step 1.4: Verify zero dependencies**
- `ga_solver/` must NOT import from `page_layout_solver/` or `models/Photo|Canvas|etc.`
- Only dependencies: `std`, `rayon`, `rand` traits, `PhantomData`
- Run: `cargo check --lib` to verify

**Step 1.5: Keep generic GA unit tests in solver.rs**
- All tests from current `ga_solver/solver.rs` stay in that file
- Tests use `NumberIndividual` and `SimpleEvolution` (domain-agnostic examples)

###Phase 2: Create Domain Trait Implementations

**Step 2.1: Create page_layout_solver.rs module file**

Create `src/solver/page_layout_solver.rs`:
```rust
//! Single-page photo layout optimization using slicing trees and genetic algorithm.
//!
//! # Architecture
//! - `solver`: Main entry point for GA-based layout optimization
//! - `individual`: LayoutIndividual - implements Individual trait
//! - `tree`: SlicingTree data structure (genotype)
//! - `affine_solver`: Layout solver (genotype → phenotype)
//! - `fitness`: Cost/fitness calculation
//! - `evolution/`: GA operators (selection, crossover, mutation)

pub mod solver;
mod individual;
mod tree;
mod affine_solver;
mod fitness;
mod evolution;

// Re-export main entry point
pub use solver::run_ga;
pub use individual::LayoutIndividual;
```

**Step 2.2: Create page_layout_solver/individual.rs**

Implement the `Individual` trait:

```rust
//! Individual implementation for photo layout optimization.

use crate::models::{Canvas, FitnessWeights, PageLayout, Photo};
use crate::solver::ga_solver::Individual;
use super::affine_solver::solve_layout;
use super::fitness::total_cost;
use super::tree::SlicingTree;

/// Individual in the GA population for photo layout optimization.
///
/// Combines a slicing tree (genotype) with its evaluated layout and fitness.
#[derive(Clone)]
pub struct LayoutIndividual {
    /// Slicing tree representing the photo arrangement structure
    pub tree: SlicingTree,
    
    /// Evaluated layout with concrete photo positions
    pub layout: PageLayout,
    
    /// Fitness score (lower is better)
    pub fitness: f64,
}

impl LayoutIndividual {
    /// Creates a new individual by evaluating a tree.
    pub fn from_tree(
        tree: SlicingTree,
        photos: &[Photo],
        canvas: &Canvas,
        weights: &FitnessWeights,
    ) -> Self {
        let layout = solve_layout(&tree, photos, canvas);
        let fitness = total_cost(&layout, photos, canvas, weights);
        Self { tree, layout, fitness }
    }
}

impl Individual for LayoutIndividual {
    type Genome = SlicingTree;
    type Phenotype = PageLayout;

    fn genome(&self) -> &Self::Genome {
        &self.tree
    }

    fn phenotype(&self) -> &Self::Phenotype {
        &self.layout
    }

    fn fitness(&self) -> f64 {
        self.fitness
    }
}
```

**Step 2.3: Update page_layout_solver/tree.rs module declarations**

The tree.rs file declares the SlicingTree data structure and its submodules (ONLY data structure concerns):

```rust
//! Slicing tree data structure for photo layout representation.
//!
//! A slicing tree is a binary tree representing a hierarchical space division.
//! Interior nodes represent cuts (Horizontal or Vertical), leaf nodes represent photos.
//!
//! # Submodules
//! - `create`: Tree construction (random_tree)
//! - `validate`: Tree validation (structural invariants)
//! - `visualize`: Tree visualization for debugging
//!
//! # Note
//! GA operators (crossover, mutation) are in `evolution/` module, not here.
//! The tree module is pure data structure + utilities.

// Tree data structure concerns only
pub mod create;       // Renamed from build.rs
pub mod validate;
pub mod visualize;

#[cfg(test)]
mod proptests;

use std::fmt;

/// Type of cut at an internal node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    V,  // Vertical
    H,  // Horizontal
}

/// Node in the slicing tree arena.
#[derive(Debug, Clone, Copy)]
pub enum Node {
    Leaf { photo_idx: u16, parent: Option<u16> },
    Internal { cut: Cut, left: u16, right: u16, parent: Option<u16> },
}

/// Slicing tree representing hierarchical space division.
#[derive(Clone)]
pub struct SlicingTree {
    nodes: Vec<Node>,
}

impl SlicingTree {
    // Core data structure methods (new, len, node access, etc.)
    // Keep existing implementation
}
```

**Step 2.4: Move tree/crossover.rs and tree/mutate.rs to evolution/ subfolder**

```bash
# Create evolution subfolder
mkdir -p src/solver/page_layout_solver/evolution

# Move GA operators from tree/ to evolution/
mv src/solver/page_layout_solver/tree/crossover.rs src/solver/page_layout_solver/evolution/crossover.rs
mv src/solver/page_layout_solver/tree/mutate.rs src/solver/page_layout_solver/evolution/mutation.rs

# Rename build.rs to create.rs
mv src/solver/page_layout_solver/tree/build.rs src/solver/page_layout_solver/tree/create.rs
```

Update tree.rs to remove crossover/mutate module declarations.

**Step 2.5: Create evolution/ subfolder modules**

**File: page_layout_solver/evolution/selection.rs** (NEW)
```rust
//! Tournament selection operator for layout individuals.

use crate::solver::page_layout_solver::individual::LayoutIndividual;
use rand::Rng;

/// Performs tournament selection on a population of layout individuals.
///
/// Randomly selects `tournament_size` individuals, picks the best, and repeats
/// `count` times to build the selected population.
pub fn tournament_select<R: Rng>(
    population: &[LayoutIndividual],
    tournament_size: usize,
    count: usize,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    // Implementation
    todo!("Extract from current selection logic")
}
```

**Note:** `crossover.rs` and `mutation.rs` already exist (moved from tree/), but update their imports:
- Change `use super::super::tree::SlicingTree` to `use crate::solver::page_layout_solver::tree::SlicingTree`

**Step 2.6: Create page_layout_solver/evolution.rs orchestrator**

**File: page_layout_solver/evolution.rs**
```rust
//! EvolutionDynamic implementation orchestrating selection, crossover, and mutation.

use crate::models::{Canvas, FitnessWeights, Photo};
use crate::solver::ga_solver::EvolutionDynamic;
use super::individual::LayoutIndividual;
use rand::Rng;

// Import GA operators from evolution/ subfolder
mod selection;
mod crossover;
mod mutation;

use selection::tournament_select;
use crossover::crossover;
use mutation::mutate;

/// Evolution strategy for photo layout optimization.
///
/// Orchestrates the three GA operators: selection, crossover, mutation.
pub struct LayoutEvolution<'a, R: Rng> {
    photos: &'a [Photo],
    canvas: &'a Canvas,
    weights: &'a FitnessWeights,
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
    rng: R,
}

impl<'a, R: Rng> LayoutEvolution<'a, R> {
    pub fn new(/* ... */) -> Self { /* ... */ }
}

impl<'a, R: Rng + Send> EvolutionDynamic<LayoutIndividual> for LayoutEvolution<'a, R> {
    fn select(&self, population: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        // Use selection::tournament_select() - already in separate file
        // Keep this function under 30 lines
    }

    fn crossover(&self, parents: &[LayoutIndividual]) -> Vec<LayoutIndividual> {
        // For each pair, apply crossover with crossover_rate
        // Use crossover::crossover() on tree genomes
        // Evaluate offspring using LayoutIndividual::from_tree()
        // If logic exceeds 30 lines, extract helper functions:
        //   - process_parent_pair()
        //   - create_offspring_from_trees()
    }

    fn mutate(&self, individuals: &mut [LayoutIndividual]) {
        // For each individual, apply mutation with mutation_rate
        // Use mutation::mutate() on tree genomes
        // Re-evaluate using LayoutIndividual::from_tree()
        // If logic exceeds 30 lines, extract helper:
        //   - mutate_and_evaluate()
    }
}
```

**Step 2.7: Rename page_layout_solver/solver.rs → affine_solver.rs**

Then create new minimal `page_layout_solver/solver.rs`:

```rust
//! High-level GA-based solver for single-page photo layout.

use crate::models::{Canvas, FitnessWeights, GaConfig, PageLayout, Photo};
use crate::solver::ga_solver::{GeneticAlgorithm, Config as GaFrameworkConfig};
use super::individual::LayoutIndividual;
use super::evolution::LayoutEvolution;
use super::tree::create::random_tree;
use rand::{rngs::StdRng, SeedableRng};

/// Runs the genetic algorithm to find an optimal photo layout.
///
/// Returns the best tree, its evaluated layout, and fitness cost.
pub fn run_ga(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    seed: u64,
) -> (SlicingTree, PageLayout, f64) {
    // Create RNG
    let mut rng = StdRng::seed_from_u64(seed);
    
    // Build initial population
    let initial_population: Vec<LayoutIndividual> = (0..ga_config.population)
        .map(|_| {
            let tree = random_tree(photos.len(), &mut rng);
            LayoutIndividual::from_tree(tree, photos, canvas, &ga_config.weights)
        })
        .collect();
    
    // Setup evolution strategy
    let evolution = LayoutEvolution::new(
        photos,
        canvas,
        &ga_config.weights,
        ga_config.tournament_size,
        ga_config.crossover_rate,
        ga_config.mutation_rate,
        rng,
    );
    
    // Map domain config to GA framework config
    let config = GaFrameworkConfig {
        population: ga_config.population,
        generations: ga_config.generations,
        mutation_rate: ga_config.mutation_rate,
        crossover_rate: ga_config.crossover_rate,
        tournament_size: ga_config.tournament_size,
        elitism_ratio: ga_config.elitism_ratio,
        timeout: ga_config.timeout,
        islands: ga_config.island_config.as_ref().map_or(1, |ic| ic.islands),
        migration_interval: ga_config.island_config.as_ref().map_or(10, |ic| ic.migration_interval),
        migrants: ga_config.island_config.as_ref().map_or(1, |ic| ic.migrants),
    };
    
    // Run GA
    let mut ga = GeneticAlgorithm::new(config, evolution);
    let best = ga.solve(initial_population).expect("GA should find solution");
    
    (best.tree.clone(), best.layout.clone(), best.fitness)
}
```

### Phase 3: File Reorganization Verification

**Step 3.1: Verify tree/ subfolder structure**

Confirm that `page_layout_solver/tree/` contains:
- `create.rs` (renamed from build.rs) - random tree generation
- `crossover.rs` - tree crossover operator
- `mutate.rs` - tree mutation operator
- `validate.rs` - tree validation
- `visualize.rs` - SVG visualization for debugging
- `proptests.rs` - property-based tests (stays in tree/ to test tree operations)

**Step 3.2: Update tree.rs module declarations**

Ensure `page_layout_solver/tree.rs` has correct submodule declarations:
```rust
pub mod create;       // Renamed from build
pub mod crossover;
pub mod mutate;
pub mod validate;
pub mod visualize;

#[cfg(test)]
mod proptests;
```

**Step 3.3: Update imports in proptests.rs**

Ensure the test file imports from the correct modules:
```rust
use crate::solver::page_layout_solver::tree::{
    create::random_tree,
    validate::validate_tree,
    SlicingTree,
};
// Import GA operators from evolution/ (they operate on trees)
use crate::solver::page_layout_solver::evolution::{
    crossover::crossover,
    mutation::mutate,
};
```

**Step 3.4: Verify module boundaries**

Ensure clean separation:
- `tree/` modules should NOT import from `evolution/`
- `evolution/` modules CAN import from `tree/` (they operate on trees)
- Both can import from `models/` (Photo, Canvas, etc.)

### Phase 4: Clean Up and Verification

**Step 4.1: Verify file structure matches target**

Double-check the final structure:
```
src/solver/
├── ga_solver.rs
├── ga_solver/
│   ├── solver.rs
│   ├── individual.rs
│   ├── evolution.rs
│   └── config.rs
├── page_layout_solver.rs
└── page_layout_solver/
    ├── solver.rs
    ├── individual.rs
    ├── tree.rs
    ├── fitness.rs
    ├── affine_solver.rs
    ├── evolution.rs
    └── tree/
        ├── create.rs
        ├── validate.rs
        ├── visualize.rs
        └── proptests.rs
    └── evolution/
        ├── selection.rs
        ├── crossover.rs
        └── mutation.rs
```

**Step 4.2: Verify function length constraint**

Check that all functions in touched files are ≤30 lines:
```bash
# Check function lengths in refactored files
for file in src/solver/ga_solver/*.rs src/solver/page_layout_solver/*.rs src/solver/page_layout_solver/tree/*.rs src/solver/page_layout_solver/evolution/*.rs; do
    echo "Checking $file"
    # Use rustfmt or manual inspection
done
```

If any function exceeds 30 lines, split it into smaller helper functions.

**Step 4.3: Run all tests**
```bash
cargo test --lib
```
All 135+ unit tests must pass.

```bash
cargo test --test integration_test
```
All 5 integration tests must pass.

**Step 4.4: Check for compilation warnings**
```bash
cargo build 2>&1 | grep -i warning
```
Resolve any unexpected warnings.

### Phase 5: Code Quality and Function Length

**Step 5.1: Review and split large functions**

For each file touched during refactoring:
1. Identify functions > 30 lines
2. Extract helper functions with clear names
3. Common patterns to extract:
   - Loop bodies → `process_item()`
   - Conditional blocks → `handle_case()`
   - Setup/teardown → `init_x()` / `finalize_x()`
   - Complex expressions → `calc_x()`

**Example: Splitting a large function**
```rust
// Before: 45 lines
fn process_population(...) {
    // 10 lines of setup
    // 20 lines of main loop
    // 15 lines of result processing
}

// After: 3 functions, each <20 lines
fn process_population(...) {
    let context = init_processing(...);
    let results = apply_operators(&context, ...);
    finalize_results(results)
}

fn init_processing(...) -> Context { ... }
fn apply_operators(...) -> Vec<Result> { ... }
fn finalize_results(...) -> FinalResult { ... }
```

**Step 5.2: Run clippy for complexity warnings**
```bash
cargo clippy -- -W clippy::cognitive_complexity
```
Address any functions flagged as too complex.

### Phase 6: Integration and Testing

**Step 6.1: Update book_layout_solver.rs**
- Change imports from `page_layout_solver::ga` to `page_layout_solver::solver`
- Verify it compiles

**Step 5.2: Run integration tests**
```bash
cargo test --test integration_test
```
All 5 tests must pass without changes.

**Step 5.3: Run unit tests**
```bash
cargo test --lib
```
Verify all 135+ tests still pass.

**Step 5.4: Run main binary**
```bash
cargo run -- -i tests/fixtures/test_photos/ -o test.typ
```
Compare output with baseline (should be identical).

### Phase 7: Documentation and Cleanup

**Step 7.1: Add module documentation**
- Each file should have `//!` module-level docs
- Explain the role in the overall architecture

**Step 7.2: Run clippy**
```bash
cargo clippy --fix
cargo clippy -- -D warnings
```

**Step 7.3: Check for dead code**
```bash
cargo build 2>&1 | grep "never used"
```

**Step 7.4: Format code**
```bash
cargo fmt
```

## Success Criteria

- [ ] ✅ All 5 integration tests pass
- [ ] ✅ All ~135 unit tests pass  
- [ ] ✅ `ga_solver/` has ZERO dependencies on `page_layout_solver/` or `models/`
- [ ] ✅ `ga_solver/` only exports: `GeneticAlgorithm`, `Individual`, `EvolutionDynamic`, `Config`
- [ ] ✅ `page_layout_solver/solver.rs` is <100 lines (just orchestration)
- [ ] ✅ Clear separation: tree operations, fitness, layout, GA traits
- [ ] ✅ **All functions ≤30 lines of code**
- [ ] ✅ Zero clippy warnings
- [ ] ✅ No dead code warnings
- [ ] ✅ All modules have documentation

## Testing Strategy

1. **After each phase**: Run `cargo test --lib` to catch regressions early
2. **After Phase 3**: Run integration tests to verify behavior unchanged
3. **After Phase 4**: Verify file structure matches target
4. **After Phase 5**: Review function lengths, split if needed
5. **After Phase 6**: Full validation (integration + unit + binary)
6. **After Phase 7**: Final docs and clippy
7. **Use git commits**: Each phase should be its own commit for easy rollback

## Risk Mitigation

- Integration tests capture current behavior (baseline)
- Refactor incrementally (phase by phase)
- Each phase should compile before moving to next
- Keep old code until new code proven working
- Use feature branches if uncertain

## Estimated Effort

- Phase 1: 30 minutes (file splitting, imports)
- Phase 2: 45 minutes (trait implementations)
- Phase 3: 30 minutes (file reorganization)
- Phase 4: 20 minutes (verification)
- Phase 5: 30 minutes (function splitting, code quality)
- Phase 6: 30 minutes (testing, validation)
- Phase 7: 15 minutes (docs, clippy)

**Total: ~3.5 hours**

## Benefits After Refactoring

1. **Clear Separation**: Generic GA vs. domain-specific photo layout
2. **Reusability**: GA framework can be used for other problems
3. **Testability**: Each component independently testable
4. **Maintainability**: Clear file boundaries, single responsibility
5. **Documentation**: Architecture reflected in file structure
6. **Type Safety**: Trait-based design enforces correct usage

## Notes

- This refactoring is **behavior-preserving** - no algorithm changes
- Integration tests ensure output remains identical
- Focus on structure, not optimization
- Can add optimizations in later phases once structure is clean
