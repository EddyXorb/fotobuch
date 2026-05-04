# Branch Review: `feat/gui-05-cross-page`

Scope: all changes relative to `main`.

---

## Bugs

### 1. Double-send in `run_rebuild_pages` — `misc.rs:60-75`

On rebuild failure for any page, `BackgroundResult::CommandFailed` is sent, **then the loop continues and `send_command_done` is unconditionally called at the end**. The UI receives both signals, causing undefined app state.

Fix: return early on first error (like every other `run_*` function), or collect errors and decide before calling `send_command_done`.

---

### 2. `run_swap_range` panics on empty slot lists — `commands/page.rs:111-112`

```rust
let (s0, s1) = (*src_slots.first().unwrap(), *src_slots.last().unwrap());
```

`src_slots` and `dst_slots` are constructed from a `Vec<usize>` passed in from the GUI. No invariant guarantees non-emptiness here — a defensive check or `?`-able error is needed.

---

### 3. `handle_delete` uses `Unplace` instead of `DeletePages` for nav-selected pages — `hotkeys.rs:153-174`

Pressing Delete on nav-selected pages emits `PendingCommand::Unplace` (photos return to pool, **empty page remains**). `PendingCommand::DeletePages` exists specifically for page removal but is not used here. This is a functional mismatch between user intent and what actually happens.

---

### 4. `build_after_command` changed guard semantics — `commands/page.rs:12-15`

Old `run_page_command`:
```rust
if move_output.changed_state.is_none() { return; }
```

New `build_after_command`:
```rust
if cmd_changed_state.is_none() && cmd_dirty.is_empty() { return; }
```

The new condition skips the build when `changed_state` is `Some` but `dirty` is accidentally empty (e.g. future callers pass `vec![]`). The original check only tested `changed_state`, which is the authoritative signal.

---

## Design / Structural Problems

### 5. `MultiSelection::Some` naming conflicts with `Option::Some`

`multi_selection.rs` names an enum variant `Some`. This shadows `Option::Some` in match arms within the same module, forcing ugly workarounds:

```rust
(std::option::Option::Some(a), std::option::Option::Some(b)) => {
```

This is exactly the problem Rust's `Option` naming guidance warns against. Rename the variant to `Active`, `Selected`, or similar.

---

### 6. `PendingCommand` and `BackgroundTask` are duplicate mirrors — `pending.rs` / `task.rs`

Every new operation adds an identical variant to both enums, plus a mechanical 1:1 mapping in `app.rs`. The only difference is `pixel_per_pt: f32`. This is dead boilerplate. Options:
- Add `pixel_per_pt` to `PendingCommand` and use it directly as `BackgroundTask`.
- Or pass `pixel_per_pt` as a separate parameter to the background channel, keeping one command type.

---

### 7. `PhotoSelection` exposes inner `MultiSelection` as `pub` — `pool.rs:4`

```rust
pub struct PhotoSelection(pub MultiSelection<String>);
```

`SlotSelection` uses a private `sel: MultiSelection<usize>` field. The inconsistency means callers can directly manipulate `PhotoSelection`'s internals. Make the field private and add methods if needed.

---

### 8. `SlotSelection::page` is `pub` — `selection.rs:5`

Direct write access to `page` bypasses any invariant between `page` and `sel`. Callers should only use the provided methods. Exposing it `pub` for read-only use in `rebuild.rs:9` is not worth the risk — add a `pub fn page(&self) -> Option<usize>` getter instead.

---

### 9. `select_all_on` directly constructs `MultiSelection::Some { .. }` — `selection.rs:49-52`

```rust
self.sel = MultiSelection::Some {
    items: (0..slot_count).collect(),
    anchor: 0,
};
```

This couples `SlotSelection` to `MultiSelection`'s internal representation. `MultiSelection` should expose a `from_range(lo..=hi)` constructor so callers don't hard-code the struct layout.

---

## Minor / Style

### 10. `is_contiguous` re-sorts already-sorted input — `drag.rs:258-261`

`src_slots` is explicitly sorted at drag-start (`drag.rs:35`). `is_contiguous` then clones the slice and sorts again. Either document the pre-sorted precondition and drop the internal sort, or rename to accept unsorted input explicitly.

### 11. `super::super::` path chains in submodules — `commands/misc.rs`, `commands/page.rs`, `commands/photo.rs`

Functions like `run_undo`, `run_page_command` etc. all use `super::super::RenderCtx<'_>` and `super::super::POOL_THUMB_MAX_EDGE_PX`. After splitting from a flat module into three submodules, these relative paths become fragile. Consider re-exporting or using `crate::background::RenderCtx` instead.

### 12. `dirty.contains(&p)` in `run_rebuild_pages` — `misc.rs:66`

Linear scan before each push. For a list of pages, use a `HashSet` or just push unconditionally (duplicates matter less than the O(n²) scan). Minor, but worth fixing for consistency with the rest of the codebase.

---

## Verification of Original Points

### #1 — Double-send in `run_rebuild_pages`: **Confirmed bug.**

On error, `CommandFailed` is sent but the loop continues to rebuild remaining pages. After the loop, `send_command_done` is unconditionally called. The UI receives both `CommandFailed` and `CommandDone` for the same logical operation.

### #2 — `run_swap_range` panics on empty slot lists: **Valid but low risk.**

`src_slots` and `dst_slots` originate from `DragSource::Slot { src_slots, .. }` captured at drag-start (`drag.rs:35`), which always contains at least the dragged slot. The `dst_slots` are computed by `compute_dst_range` which returns `None` for empty ranges. The only caller (`complete_slot_drag`) guards with `is_contiguous(&src_slots)` and the `compute_dst_range` check. In practice the panic is unreachable from the current call site — but the unwrap is still brittle against future callers.

### #3 — `handle_delete` uses `Unplace` for nav pages: **Not a bug — intentional.**

Commit `6949af2` explicitly documents: "Delete key on nav-panel selection now iterates selected pages and emits Unplace commands for all filled slots, preserving the page structure." The design intent is to clear page content without removing pages — page deletion is a more destructive action not mapped to the simple Delete key. `PendingCommand::DeletePages` exists for programmatic use but was deliberately excluded from the hotkey.

### #4 — `build_after_command` changed guard semantics: **Not a bug.**

The condition `if cmd_changed_state.is_none() && cmd_dirty.is_empty()` only short-circuits when **both** are absent. If `changed_state` is `Some` but `dirty` is empty, the code still proceeds to `build()`, which is correct — `build()` independently determines which pages need compilation. The original single check (`changed_state.is_none()`) was arguably too conservative.

### #5 — `MultiSelection::Some` naming: **Valid style issue.**

The workaround `std::option::Option::Some(a)` appears in `range_to_ordered` (line 67). It works but is a readability trap.

### #6 — `PendingCommand` / `BackgroundTask` duplication: **Valid design debt, low urgency.**

The 1:1 mapping in `dispatch_commands` (`app.rs:174-270`) is purely mechanical. Worth addressing eventually but the current pattern is at least straightforward and grep-able.

### #7 — `PhotoSelection(pub MultiSelection<String>)`: **Confirmed.**

`SlotSelection` properly encapsulates its `sel` field as private; `PhotoSelection` exposes the inner `MultiSelection` as `pub`. No external code currently pokes at `.0` directly outside of the module, but the inconsistency invites it.

### #8 — `SlotSelection::page` is `pub`: **Minor risk, acceptable for now.**

All 7 usages of `.page` in the GUI are read-only (`.page == Some(x)`, `.page.is_some()`, `if let Some(page) = .page`). None write to it. A getter would be cleaner but the risk is contained.

### #9 — `select_all_on` constructs `MultiSelection::Some { .. }` directly: **Valid.**

A `from_range` constructor on `MultiSelection` would decouple `SlotSelection` from the enum's internal representation.

### #10 — `is_contiguous` re-sorts: **Already fixed.**

Commit `2d1fe40` added `src_slots.sort_unstable()` at drag-start. However `is_contiguous` (`drag.rs:258`) still clones and sorts again — the redundant sort remains.

### #11 — `super::super::` path chains: **Valid style issue.** Low priority.

### #12 — `dirty.contains(&p)` linear scan: **Valid.** Also harmless given page counts are typically < 100.

---

## Additional Issues Found

### 13. Nav-drag Move dispatches multiple independent `Move` commands — `drag.rs:180-195`

When multiple nav pages are selected and dragged in Move mode to a target page, a separate `PendingCommand::Move` is emitted per source page. Each command runs independently on the background thread, and intermediate state changes (page indices shifting) are not accounted for. If pages [2, 4] are moved to page 6, after moving page 2's slots, page indices shift — page 4 may no longer refer to the intended page. These should be batched into a single compound command.

### 14. German strings remain in `add_dialog.rs`

Commit `f7f38a7` ("translate German UI strings to English") missed the add dialog:
- `"Fotos hinzufügen"` (window title)
- `"Noch nicht implementiert."` (label)
- `"Schließen"` (button)

### 15. `PhotoSelection::is_selected` allocates on every call — `pool.rs:33`

```rust
pub fn is_selected(&self, id: &str) -> bool {
    self.0.is_selected(&id.to_owned())
}
```

`BTreeSet<String>` supports `contains` via `Borrow<str>`, but `MultiSelection<K>::is_selected(&K)` requires an owned `&String`. This forces a heap allocation per call. Called once per photo row per frame — measurable overhead with large pools. Fix: add a `is_selected_ref` method to `MultiSelection` that accepts `&Q where K: Borrow<Q>`, or specialize for `String`.

### 16. `handle_command_done` clears slot selection unconditionally — `app.rs:134`

```rust
self.state.interaction.selections.slots.clear();
```

Every successful command (including `ConfigSet`, `ReleaseBuild`, unrelated rebuilds) clears the user's slot selection. This makes multi-step workflows frustrating — e.g. select slots, rebuild, selection is gone. The clear should be conditional on commands that actually modify page structure.

### 17. `run_rebuild_pages` — `send_command_done` uses potentially stale `new_state` after error — `misc.rs:73`

After the loop, if some pages succeeded and some failed, `new_state` holds the last successful rebuild's state, but earlier successful rebuilds may have also changed state. The accumulated `dirty` list reflects all successes, but `new_state` only has the latest one. If earlier rebuilds changed state in ways the last one didn't preserve, data is lost. Tied to issue #1 — once the early-return is added, this becomes moot.

### 18. `mark_dirty` fills **all** pages for most commands — `app.rs:338-359`

`MoveToNewPage`, `MovePage`, `SwapRange`, `Unplace`, `DeletePages`, `RebuildPages`, `RebuildAll`, `ReleaseBuild` all hit `dirty.fill(true)`. This means the entire page cache is marked stale even for single-page operations. This is functionally correct (conservative), but causes unnecessary visual flicker for the unaffected pages. The commands that know their affected pages (e.g. `RebuildPages { pages }`, `Unplace { page, .. }`) could mark only those.
