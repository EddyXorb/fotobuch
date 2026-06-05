# How the Layout Engine Works (Advanced)

fotobuch lays out a book in **two stages**, each solved by its own algorithm:

1. The **book layout solver** decides *how many photos go on which page* and
   *which photos belong together* — a Mixed Integer Program (MIP) refined by a
   local search.
2. The **page layout solver** decides *how the photos on a single page are
   arranged* — a genetic algorithm operating on *slicing trees*.

This page explains the ideas behind both, including a novel contribution that does
not appear in the published literature. It is background reading — you never need
to understand any of this to use fotobuch. For practical knobs, see
[Solver Tuning](solver-tuning.md).

## Stage 1 — Book layout solver (page assignment)

Given a chronologically ordered, grouped sequence of photos, the book layout
solver partitions it into pages. A page is a contiguous slice of the sequence, so
the whole problem reduces to choosing **cut points** in the sequence.

It runs in two phases:

- **MIP phase.** The assignment is formulated as a Mixed Integer Program and
  solved with [HiGHS](https://highs.dev/) via `good_lp`. The objective balances
  the target page count, keeping [photo groups](../glossary.md#photo-group)
  coherent, and respecting per-page photo limits. Hard constraints (page count,
  photos per page, groups per page, minimum group share on a split) are enforced
  exactly.
- **Local search phase.** The MIP optimizes a proxy objective; the local search
  then moves cut points based on the *actual* rendered layout quality, targeting
  pages with too much white space first (worst-first). A layout cache prevents
  redundant page-solver calls.

For very large books (300+ photos) the sequence is automatically decomposed into
independent sub-problems so the solver stays within its time budget.

## Stage 2 — Page layout solver (slicing-tree GA)

Arranging the photos *within* a page is the hard, visually visible part. fotobuch
builds on the slicing-tree genetic algorithm described in:

> O. Fan, *"Photo Layout with a Fast Evaluation Method and Genetic Algorithm"*,
> IEEE ICMEW 2012.
> [IEEE Xplore](https://ieeexplore.ieee.org/document/6266273).

A big thank-you also to [@masse](https://github.com/masse) for
[collage-solver](https://github.com/masse/collage-solver), whose work was an
inspiring starting point.

### Slicing trees

A page layout is encoded as a full binary tree:

- **Leaves** are photos.
- **Internal nodes** are cuts: `V` (vertical cut, children side by side) or `H`
  (horizontal cut, children stacked).

For *N* photos the tree has *N* leaves and *N−1* internal nodes. This structure
*guarantees* — without any cost term — that slots align along cut lines, gaps are
uniform, and nothing overlaps. The genetic algorithm only has to evolve the tree
**topology** and the **cut directions**.

### The genetic algorithm

- **Population & islands.** Several independent populations evolve in parallel on
  separate threads (island model), periodically migrating their best individuals.
  This needs no locking during evolution and converges better than a single large
  population.
- **Mutation** flips a single cut (`V ↔ H`), which can change a page's appearance
  dramatically.
- **Crossover** swaps two compatible subtrees between parents, creating genuinely
  new topologies.
- **Cost function** balances coverage (minimize white space), how closely each
  photo's area matches its [weight](../glossary.md#photo-weight), and optional
  centering — while [aspect ratios](../glossary.md#aspect-ratio) are always kept
  intact.

## Two contributions beyond the paper

fotobuch extends the published algorithm in two ways that materially improve both
speed and result quality.

### 1. Exact gap computation in O(N)

The original algorithm either approximates the inter-photo gap (β) or recomputes
it in **O(N³)** per fitness evaluation. fotobuch derives an exact closed-form
solution instead.

With a gap, the relationship between a node's width and height becomes **affine**:
`w = α·h + γ`. Each node carries a coefficient pair `(α, γ)` that is propagated
bottom-up through the tree:

- **Leaf** with aspect ratio `a`: `α = a`, `γ = 0`
- **V-node** (children share height, widths add):
  `α = αₗ + αᵣ`, `γ = γₗ + γᵣ + β`
- **H-node** (children share width, heights add):
  `α = αₗ·αᵣ / (αₗ + αᵣ)`, `γ = (γₗ/αₗ + γᵣ/αᵣ − β)·αₗ·αᵣ / (αₗ + αᵣ)`

A single top-down pass then assigns exact dimensions and positions. Because
`α > 0` is provably invariant for every node, the computation is always
well-defined. This reduces the per-evaluation cost from **O(N³) to O(N)** while
guaranteeing pixel-accurate placement with the *precise* gap that fills the page
with no overlap or leftover space. This formulation does not appear in the
literature.

### 2. Reading-order preservation via DFS indexing

A photo book tells a story, so the visual order on a page should match the
chronological order of the photos. Instead of paying a fitness penalty to *nudge*
the algorithm toward this, fotobuch makes it **structural**: photos are assigned
to leaves in depth-first pre-order. A `V` cut visits left before right, an `H` cut
visits top before bottom — so the oldest photo always lands top-left and the
sequence flows naturally to the bottom-right.

This makes correct reading order impossible to violate and removes a whole cost
term. It also enables a cheaper mutation: a cut flip changes the spatial layout
but never the depth-first leaf order, so no re-assignment is needed. You can turn
this off with `enforce_order: false` (see [Solver Tuning](solver-tuning.md)).

## Why it matters

Together these two ideas mean fotobuch evaluates far more candidate layouts per
second *and* keeps your story in order by construction — which is why its
many-photos-per-page results tend to look better than those of off-the-shelf
tools.
