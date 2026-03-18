# Incremental Build Change Detection: `compute_outdated_pages()`

## Objective

Determine which pages need re-rendering when project state changes. A page is unchanged only if:

- No photo metadata changes (aspect ratio, area_weight)
- Page slot structure matches previous state (index-coupled to photos)
- Each slot's aspect ratio matches its corresponding photo's aspect ratio

## Algorithm

### Phase 1: Build Reference Maps

**Map 1 - Photo Metadata** (reference state)

```rust
HashMap<PhotoId, (AspectRatio, AreaWeight)>
```

Fast lookup for detecting photo metadata mutations.

**Map 2 - Page Hashes** (reference state)

```rust
HashMap<BTreeSet<PhotoId>, Vec<usize>>
  Key:   BTreeSet of all photo IDs on that page
  Value: Vec of indices in reference.layout where this photo set appears
```

Handles page reordering and duplicate photo sets.

**Set 1 - Changed Photos** (reference → new comparison)

```rust
HashSet<PhotoId>
```

Collect photos with changed metadata (aspect ratio or area_weight).

### Phase 2: Evaluate Each New Page

For each page in `new.layout`:

1. **Check photo mutation**: If any photo in `page.photos` is in `changed_photos` → **OUTDATED**, skip.

2. **Find matching old page**:
   - Create `BTreeSet<PhotoId>` from this page's photos
   - Look up in page_hashes to get `Vec<usize>` of candidate old indices
   - If empty → **OUTDATED**, skip
   - For each candidate index:
     - Compare `new_page.slots` with `old_page.slots` (exact equality: x, y, width, height)
     - If match found: use this old page for remaining checks
     - Else after all candidates: → **OUTDATED**, skip

3. **Validate slot structure**:
   - Check `new_page.slots.len() == new_page.photos.len()`
   - If mismatch → **OUTDATED**, skip

4. **Validate aspect ratios**:
   - For each index `i` in `new_page.photos`:
     - `slot_aspect_ratio = slots[i].width_mm / slots[i].height_mm`
     - `photo_aspect_ratio = photo_metadata[i].aspect_ratio`
     - If `|slot_ar - photo_ar| > THRESHOLD` → **OUTDATED**, skip

5. If all checks pass → **UNCHANGED**

## Key Design Decisions

- **BTreeSet page identity**: Handles page reordering within the layout
- **Candidate indices**: Supports reference states with duplicate photo sets (though unlikely)
- **Slot exact-match**: Detects any layout structural mutation (re-solving)
- **Index coupling**: Assumes `slots[i]` always corresponds to `photos[i]`

## Implementation Location

Refactor `src/state_manager.rs`:
- Keep `src/state_manager.rs` as the module root
- Create `src/state_manager/page_change_detection.rs` - new submodule
- Declare `mod page_change_detection;` in `state_manager.rs`

`page_change_detection.rs` contains `compute_outdated_pages()` and helpers. Called from `state_manager.rs`.

## Testing Strategy

### Unit Tests

**Unchanged pages**:

- Reference and new are identical → no outdated pages
- All photos unchanged, all slots unchanged, all ARs match

**Photo metadata mutations**:

- One photo's aspect ratio changes → its page becomes outdated
- One photo's area_weight changes → its page becomes outdated
- Multiple photos on same page with metadata changes → page outdated

**Slot structure mutations**:

- One slot's position (x, y) changes → page outdated (even if AR matches)
- One slot's dimensions (width, height) change → page outdated
- All slots structurally identical but photos reordered → page outdated (photos != slots)

**Aspect ratio mismatch**:

- New photo AR differs from slot AR beyond threshold → page outdated
- Multiple mismatched slots on same page → page outdated
- AR just within tolerance → page unchanged

**Photo list changes**:

- Photo added to page → BTreeSet differs, page outdated
- Photo removed from page → BTreeSet differs, page outdated
- Photos reordered within page → BTreeSet same, but slots may differ

**Page reordering**:

- Reference: page 1 [A,B], page 2 [C,D]
- New: page 1 [C,D], page 2 [A,B]
- Result: Both pages unchanged (content identity preserved)

**Duplicate photo sets in reference**:

- Reference: page 1 [A,B], page 2 [A,B]
- New: page 1 [A,B], page 2 [C,D]
- Result: Page 1 unchanged (matches via slots), page 2 outdated (no candidate match)

**Edge cases**:

- Empty pages (0 photos) → slot count mismatch
- Single-photo pages → minimal test case
- Reference state completely empty → all new pages outdated
- New state completely empty → no pages to evaluate
