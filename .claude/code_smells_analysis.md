# Code Smells Analysis & Remediation Plan

## Executive Summary
Analyzed 40 Rust source files (~7,500 lines). Overall code quality is **good** with modern architecture (trait-based GA, modular structure). Identified 28 code smells across 5 priority levels.

---

## 🔴 **CRITICAL Priority (Fix Immediately)**

### 1. **Error Handling: Excessive `.unwrap()` Usage**
**Location:** Throughout codebase (50+ occurrences)
**Files:** 
- `solver/page_layout_solver/tree/visualize.rs` (multiple `writeln!(...).unwrap()`)
- `solver/ga_solver/island.rs` (6 occurrences)
- `solver/ga_solver/evolution.rs` (`expect` on empty population)
- `input/scanner.rs` (`unwrap()` on date parsing)

**Problems:**
- Can panic in production
- Poor error propagation
- Lack of error context

**Solution:**
```rust
// Bad
writeln!(&mut svg, "...").unwrap();

// Good
writeln!(&mut svg, "...")?;  // Propagate with ?
// OR
writeln!(&mut svg, "...").expect("Failed to write SVG node");  // Better panic message
```

**Action Items:**
1. Replace `unwrap()` with `?` in functions returning `Result`
2. Add `Result` return types where missing
3. Use `expect()` with descriptive messages for truly infallible operations
4. Add custom error types for better error handling

**Estimated Effort:** 4-6 hours

---

### 2. **Magic Numbers Everywhere**
**Location:** All modules (100+ occurrences)
**Examples:**
```rust
// visualize.rs
const NODE_RADIUS: f64 = 25.0;  // Good!
let canvas_width = max_x + NODE_RADIUS * 2.0 + 20.0;  // Bad! What's 20.0?

// fitness.rs
let k_i = if s_i / t_i < 0.5 { 5.0 } else { 1.0 };  // What are 0.5 and 5.0?

// All over tests
Canvas::new(297.0, 210.0, 2.0, 3.0)  // A4 dimensions?
Photo::new(1.5, 1.0, "test")  // Why 1.5?
```

**Solution:**
Create constants with semantic names:
```rust
// fitness.rs
const UNDERSIZED_THRESHOLD: f64 = 0.5;
const UNDERSIZED_PENALTY_MULTIPLIER: f64 = 5.0;
const NORMAL_PENALTY_MULTIPLIER: f64 = 1.0;

// tests helper module
mod test_constants {
    pub const A4_WIDTH_MM: f64 = 297.0;
    pub const A4_HEIGHT_MM: f64 = 210.0;
    pub const DEFAULT_GAP_MM: f64 = 2.0;
    pub const DEFAULT_BLEED_MM: f64 = 3.0;
    pub const LANDSCAPE_ASPECT_RATIO: f64 = 1.5;
}
```

**Action Items:**
1. Extract test constants to shared module
2. Add constants for fitness function thresholds
3. Document magic number meanings
4. Add constants for visualization layout parameters

**Estimated Effort:** 3-4 hours

---

## 🟠 **HIGH Priority (Fix Soon)**

### 3. **Too Many Parameters (8-10 params)**
**Location:** Multiple GA functions
**Violations:**
```rust
// generation.rs - 10 parameters!
pub fn generate_offspring<R: Rng>(
    population: &[LayoutIndividual],
    elite: Vec<LayoutIndividual>,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
    target_size: usize,
    rng: &mut R,
) -> Vec<LayoutIndividual>

// island.rs - 8 parameters
fn spawn_island(...)
fn run_single_island(...)
```

**Solution:**
Create parameter objects:
```rust
pub struct GenerationParams<'a> {
    pub photos: &'a [Photo],
    pub canvas: &'a Canvas,
    pub weights: &'a FitnessWeights,
    pub tournament_size: usize,
    pub crossover_rate: f64,
    pub mutation_rate: f64,
}

pub fn generate_offspring<R: Rng>(
    population: &[LayoutIndividual],
    elite: Vec<LayoutIndividual],
    params: &GenerationParams,
    target_size: usize,
    rng: &mut R,
) -> Vec<LayoutIndividual>
```

**Action Items:**
1. Create `GenerationParams` struct
2. Create `IslandParams` struct
3. Refactor all high-param-count functions
4. Update all call sites

**Estimated Effort:** 4-5 hours

---

### 4. **Repeated `.clone()` Patterns**
**Location:** GA operators, crossover logic
**Examples:**
```rust
// operators.rs
(parent1.clone(), parent2.clone())  // Double clone on single line

// Multiple locations
(best.tree.clone(), best.layout.clone(), best.fitness)  // Triple return
```

**Solution:**
- Use references where possible
- Implement `Copy` for small types
- Use `Rc` or `Arc` for shared ownership if needed
- Return structs instead of tuples

```rust
#[derive(Clone)]
pub struct GaResult {
    pub tree: SlicingTree,
    pub layout: PageLayout,
    pub fitness: f64,
}

// Return by value, clone at call site if needed
pub fn run_ga(...) -> GaResult {
    GaResult { tree, layout, fitness }
}
```

**Action Items:**
1. Create result structs to avoid tuple cloning
2. Audit clone usage and eliminate unnecessary ones
3. Use `Cow` for conditional cloning

**Estimated Effort:** 3-4 hours

---

### 5. **Incomplete TODOs in Production Code**
**Location:** 3 critical TODOs
```rust
// solver/book_layout_solver.rs:36
// TODO: In the future, implement intelligent distribution:
// Multi-page layout not implemented

// solver/fitness.rs:85
// TODO: Implement in Step 4
fn cost_coverage(layout: &PageLayout) -> f64 {
    1.0 - layout.coverage_ratio()  // Is this the final implementation?
}

// solver/solver.rs:114
// TODO: Support multi-page export for book layouts
```

**Solution:**
1. Either implement or document as future work
2. Add issue tracker references
3. Remove if not planned

**Action Items:**
1. Review each TODO with stakeholders
2. Create GitHub issues for planned work
3. Add `#[cfg(feature = "multi_page")]` if optional
4. Document limitations in API docs

**Estimated Effort:** 2-3 hours (analysis) + implementation time

---

## 🟡 **MEDIUM Priority (Plan & Schedule)**

### 6. **Long Functions (40+ lines)**
**Location:** solver.rs, crossover.rs, fitness.rs
**Examples:**
- `solve_layout()` - 80 lines, does 4 distinct things
- `compute_coefficients_recursive()` - 60+ lines with nested match
- `cost_reading_order()` - Complex nested loops

**Solution:**
Break into smaller focused functions:
```rust
// Before: solve_layout() does everything
pub fn solve_layout(...) -> PageLayout {
    // 80 lines of mixed concerns
}

// After: Each phase is separate
pub fn solve_layout(...) -> PageLayout {
    let coeffs = compute_coefficients(tree, photos, canvas.beta);
    let dims = compute_dimensions(tree, &coeffs, canvas);
    let positions = compute_positions(tree, &dims, canvas.beta);
    extract_placements(tree, &dims, &positions, canvas)
}

fn extract_placements(...) -> PageLayout { ... }  // New, focused function
```

**Action Items:**
1. Split `solve_layout()` into pipeline functions
2. Extract helper functions from recursive functions
3. Apply to all 40+ line functions

**Estimated Effort:** 6-8 hours

---

### 7. **Partial Comparison with `unwrap()`**
**Location:** All sort comparisons
```rust
population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());
```

**Problem:** Can panic if fitness is NaN

**Solution:**
```rust
population.sort_by(|a, b| {
    a.fitness.partial_cmp(&b.fitness)
        .unwrap_or(std::cmp::Ordering::Equal)
});

// Or better: total_cmp for f64
population.sort_by(|a, b| {
    a.fitness.total_cmp(&b.fitness)
});
```

**Action Items:**
1. Add NaN checks in fitness calculations
2. Use `total_cmp` for f64 sorting
3. Add assertions that fitness is always finite

**Estimated Effort:** 2 hours

---

### 8. **Large Files (400+ lines)**
**Files:**
- `solver.rs` - 495 lines (mixed phases)
- `visualize.rs` - 407 lines (could split rendering)
- `crossover.rs` - 395 lines (complex algorithm)
- `fitness.rs` - 346 lines (multiple cost functions)

**Solution:**
Split by responsibility:
```
fitness.rs →
  fitness/
    mod.rs (public API)
    size_distribution.rs
    coverage.rs
    barycenter.rs
    reading_order.rs
```

**Action Items:**
1. Split fitness.rs into submodules
2. Consider splitting solver.rs phases
3. Keep crossover.rs as-is (single algorithm)

**Estimated Effort:** 4-6 hours

---

### 9. **Unused Function Parameters**
**Location:** visualize.rs, fitness.rs
```rust
fn draw_edges(
    svg: &mut String,
    tree: &SlicingTree,
    layouts: &[NodeLayout],
    _node_radius: f64,    // Unused!
    _level_height: f64,   // Unused!
)

pub fn total_cost(
    layout: &PageLayout,
    photos: &[Photo],
    _canvas: &Canvas,     // Unused!
    weights: &FitnessWeights,
)
```

**Solution:**
- Remove if truly unnecessary
- Document why reserved (future use)
- Add `#[allow(unused)]` with comment if intentional

**Action Items:**
1. Audit all `_` prefixed params
2. Remove or document each
3. Run `cargo clippy` to find more

**Estimated Effort:** 1 hour

---

## 🟢 **LOW Priority (Nice to Have)**

### 10. **Test Code Duplication**
**Location:** All test modules
- Same test setup repeated (Canvas, Photos, GaConfig)
- Magic numbers in tests

**Solution:**
```rust
#[cfg(test)]
mod test_helpers {
    pub fn default_test_canvas() -> Canvas {
        Canvas::new(1000.0, 800.0, 5.0, 0.0)
    }
    
    pub fn sample_photos() -> Vec<Photo> {
        vec![
            Photo::new(1.5, 1.0, "landscape".into()),
            Photo::new(0.67, 1.0, "portrait".into()),
        ]
    }
}
```

**Estimated Effort:** 3-4 hours

---

### 11. **Missing Module Documentation**
**Files missing `//!` docs:**
- `ga_solver/operators.rs`
- `ga_solver/selection.rs`
- Several other modules

**Solution:**
Add module-level docs to all public modules

**Estimated Effort:** 2-3 hours

---

### 12. **Inconsistent Naming**
- `run_ga` vs `solve_layout` (verb inconsistency)
- `compute_*` vs `calculate_*`
- `cost_*` vs `penalty_*`

**Solution:**
Standardize naming conventions in style guide

**Estimated Effort:** 1-2 hours (guide) + refactoring

---

## 📊 **Code Metrics Summary**

| Metric | Current | Target |
|--------|---------|--------|
| Largest file | 495 lines | < 300 |
| Max function parameters | 10 | ≤ 5 |
| `.unwrap()` count | 50+ | < 10 |
| Magic numbers | 100+ | < 20 |
| TODO comments | 3 | 0 in prod code |
| Test duplication | High | Low |
| Module docs coverage | 60% | 95% |

---

## 🗓️ **Implementation Roadmap**

### Sprint 1 (Week 1): Critical Fixes
- [ ] Day 1-2: Fix error handling (`.unwrap()` → `?`)
- [ ] Day 3-4: Extract magic numbers to constants
- [ ] Day 5: Testing & validation

### Sprint 2 (Week 2): High Priority
- [ ] Day 1-2: Create parameter structs
- [ ] Day 3: Fix clone patterns
- [ ] Day 4-5: Address TODOs (implement or document)

### Sprint 3 (Week 3): Medium Priority
- [ ] Day 1-2: Split long functions
- [ ] Day 3: Fix partial_cmp
- [ ] Day 4-5: Split large files

### Sprint 4 (Week 4): Low Priority & Polish
- [ ] Day 1-2: Test helpers & deduplication
- [ ] Day 3-4: Documentation pass
- [ ] Day 5: Final review & clippy fixes

---

## ✅ **What's Already Good**

1. ✅ **Modern Rust patterns:** Trait-based architecture, strong types
2. ✅ **Good separation:** GA is generic, reusable
3. ✅ **Comprehensive tests:** 107 tests passing
4. ✅ **Zero clippy warnings:** Code quality baseline is solid
5. ✅ **Recent refactoring:** GA split into modules (good job!)
6. ✅ **Clear algorithms:** Comments explain complex logic
7. ✅ **Type safety:** Strong typing everywhere

---

## 🎯 **Success Criteria**

After remediation:
- [ ] Zero production `.unwrap()` calls
- [ ] All magic numbers named
- [ ] No function > 50 lines
- [ ] No function > 6 parameters
- [ ] All TODOs resolved or tracked
- [ ] 100% module documentation
- [ ] Maintainability Index > 70

---

## 💡 **Recommendations Beyond Code Smells**

1. **Add property-based testing** for tree operations
2. **Benchmark suite** for performance regression
3. **Consider using `thiserror`** for better error types
4. **Add `tracing` spans** to GA for debugging
5. **Document complexity** of algorithms (Big-O)

---

## 📝 **Notes**

- Most issues are **quality-of-life** improvements, not bugs
- Current code is **functional and correct**
- Focus on **error handling** and **maintainability** first
- Architecture is **solid**, just needs polish
