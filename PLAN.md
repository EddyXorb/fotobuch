# Fix Plan for `feat/gui-05-cross-page`

## Priority 1 — Bugs

### A. Fix double-send in `run_rebuild_pages` (#1, #17)

**File:** `gui/background/commands/misc.rs:57-75`

Return early on first error. Drop the loop-and-accumulate pattern:

```rust
pub fn run_rebuild_pages(pages: Vec<usize>, rctx: &mut RenderCtx<'_>) {
    let mut dirty: Vec<usize> = vec![];
    let mut new_state: Option<ProjectState> = None;
    for p in pages {
        match rebuild(rctx.project_root, RebuildScope::SinglePage(p)) {
            Err(e) => {
                let _ = rctx.result_tx.send(BackgroundResult::CommandFailed(
                    format!("rebuild page {p}: {e}"),
                ));
                return; // <-- early return, no send_command_done
            }
            Ok(out) => {
                dirty.push(p);
                if let Some(s) = out.changed_state {
                    new_state = Some(s);
                }
            }
        }
    }
    send_command_done(new_state, dirty, rctx);
}
```

### B. Fix nav-drag multi-page Move index corruption (#13)

**File:** `gui/app/input_handler/drag.rs:178-195`

Problem: Multiple `PendingCommand::Move` per source page — after first executes, indices shift.

Fix: Emit a single `PendingCommand::MoveAllSlots { sources: Vec<usize>, dst_page: usize }` that the background handler processes in one transaction (reverse-sorted source pages to keep indices stable), or simply restrict nav Move-to-page to single-page selection only.

Simpler option (restrict): Only emit the `Move` for `src_page` (the drag origin), ignore `src_pages` — same as Swap mode already does. Multi-page move-to-target is not well-defined UX anyway.

### C. Conditional slot selection clear (#16)

**File:** `gui/app.rs:134`

Replace unconditional `selections.slots.clear()` — only clear when new_state's layout changed (page count differs or page slot counts differ from before):

```rust
if new_state.layout.len() != old_layout_len {
    self.state.interaction.selections.slots.clear();
}
```

Or: track which command is in-flight and only clear for structural commands (Move, Delete, Unplace, Place).

---

## Priority 2 — Defensive / Correctness

### D. Guard `run_swap_range` against empty vecs (#2)

**File:** `gui/background/commands/page.rs:111-112`

Replace `.unwrap()` with early return + `CommandFailed`:

```rust
let (Some(s0), Some(s1)) = (src_slots.first(), src_slots.last()) else {
    let _ = rctx.result_tx.send(BackgroundResult::CommandFailed("empty slot list".into()));
    return;
};
```

### E. Rename `MultiSelection::Some` → `MultiSelection::Active` (#5)

**Files:** `gui/state/multi_selection.rs`, all match sites.

Rename the variant. Remove need for `std::option::Option::Some` disambiguation in `range_to_ordered`.

### F. Add `MultiSelection::from_range` constructor (#9)

**File:** `gui/state/multi_selection.rs`

```rust
impl MultiSelection<usize> {
    pub fn from_range(range: std::ops::RangeInclusive<usize>) -> Self {
        Self::Active { items: range.collect(), anchor: *range.start() }
    }
}
```

Use in `SlotSelection::select_all_on`.

---

## Priority 3 — Performance / Style

### G. Fix `PhotoSelection::is_selected` allocation (#15)

**File:** `gui/state/multi_selection.rs` + `gui/state/pool.rs`

Add to `MultiSelection`:
```rust
pub fn contains<Q>(&self, k: &Q) -> bool
where K: std::borrow::Borrow<Q>, Q: Ord + ?Sized {
    match self { Self::Active { items, .. } => items.contains(k), _ => false }
}
```

Change `PhotoSelection::is_selected` to call `self.0.contains(id)`.

### H. Make `PhotoSelection` field private (#7)

**File:** `gui/state/pool.rs`

Change `pub struct PhotoSelection(pub MultiSelection<String>)` → private inner. Audit callers — currently none access `.0` externally.

### I. Remove redundant sort in `is_contiguous` (#10)

**File:** `gui/app/input_handler/drag.rs:258`

`src_slots` is sorted at drag-start. Change `is_contiguous` to assert/document the precondition and remove internal sort:

```rust
fn is_contiguous(sorted_slots: &[usize]) -> bool {
    debug_assert!(sorted_slots.windows(2).all(|w| w[0] <= w[1]));
    sorted_slots.windows(2).all(|w| w[1] == w[0] + 1)
}
```

### J. Translate remaining German strings (#14)

**File:** `gui/app/widgets/add_dialog.rs`

- `"Fotos hinzufügen"` → `"Add photos"`
- `"Noch nicht implementiert."` → `"Not yet implemented."`
- `"Schließen"` → `"Close"`

### K. Refine `mark_dirty` for targeted commands (#18)

**File:** `gui/app.rs:320-360`

For `Unplace { page, .. }` mark only `page`. For `RebuildPages { pages, .. }` mark only those pages. Keep `dirty.fill(true)` for commands that can change page count (`DeletePages`, `MoveToNewPage`, `MovePage`, `RebuildAll`, `ReleaseBuild`).

### L. Merge `PendingCommand` and `BackgroundTask` (#6)

**Files:** `gui/app/pending.rs`, `gui/task.rs`, `gui/app.rs`, `gui/background.rs`

Eliminate `PendingCommand` entirely. Add `pixel_per_pt: f32` as a field to `BackgroundTask` variants that need it (all except `LoadPhotoThumbnails`). `input_handler::handle` and `widgets` return `HashSet<BackgroundTask>` directly. Delete `dispatch_commands` — the loop in `FotobuchApp::ui` becomes:

```rust
for mut task in cmds {
    task.set_pixel_per_pt(ppt);  // helper method on BackgroundTask
    mark_dirty(&mut self.state.data.pages.dirty, &task);
    let _ = self.task_tx.send(task);
}
```

`BackgroundTask` needs `#[derive(Debug, Hash, PartialEq, Eq)]`. `f32` doesn't impl `Hash`/`Eq` — store `pixel_per_pt` separately: either pass it through the channel as a second field in a wrapper `struct TaskMessage { task: BackgroundTask, pixel_per_pt: f32 }`, or store it on `RenderCtx` and send a separate `SetPixelPerPt(f32)` message before each batch. Simplest: wrap in `struct TaskEnvelope(BackgroundTask, f32)` on the send side only, keep `BackgroundTask` `Hash`-free and use a newtype `HashSet` wrapper or `Vec` with dedup.

**Recommended approach:** Keep `PendingCommand` but make it a type alias or thin wrapper:
1. Remove `pixel_per_pt` from `BackgroundTask` variants — store on `RenderCtx` (already done).
2. Make `BackgroundTask` = current `PendingCommand` (rename).
3. `dispatch_commands` only does `mark_dirty` + `task_tx.send` — no mapping needed.

### M. Replace `super::super::` with crate-level re-exports (#11)

**Files:** `gui/background/commands/misc.rs`, `gui/background/commands/page.rs`, `gui/background/commands/photo.rs`

Add to `gui/background.rs`:
```rust
pub(crate) use render::send_command_done;
// RenderCtx is already pub(super), promote to pub(crate)
```

Then in submodules, replace `super::super::RenderCtx<'_>` with `crate::background::RenderCtx<'_>` and `super::super::render::send_command_done` with `crate::background::send_command_done`. Same for `POOL_THUMB_MAX_EDGE_PX`.

---

## Deferred / Won't Fix Now

- **#8** (`SlotSelection::page` pub): All usages read-only. Defer.
- **#12** (`dirty.contains` O(n²)): Negligible with <100 pages. Defer.
