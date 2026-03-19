# `StateManager::refresh_sources` Design

## Problem

Three build paths (`incremental_build`, `multipage_build`, `rebuild`) each independently call `ensure_previews`. None of them update `ProjectState` when a source file is replaced on disk with a new file of the same name — leaving stale metadata that breaks duplicate detection, change detection, and layout solving (wrong aspect ratio).

The following fields of `PhotoFile` are derived from the file and can become stale: `hash`, `width_px`, `height_px`, `timestamp`. `area_weight` is user-defined and must not be overwritten.

## Solution

A `StateManager` method `refresh_sources` that:
1. Regenerates stale previews (existing logic)
2. Re-reads all file-derived metadata for sources whose previews were regenerated
3. Updates `self.state` photo entries in place
4. Returns a combined result

All three build paths replace their boilerplate with a single call.

## Type Changes

### `PreviewCacheResult` — extended

Add the set of photo IDs whose previews were regenerated (= sources that changed on disk):

```rust
pub struct PreviewCacheResult {
    pub created: usize,
    pub skipped: usize,
    pub total: usize,
    pub regenerated_ids: Vec<String>,  // new
}
```

`regenerated_ids` is populated in the `else` branch of the per-photo loop, where a preview is already known to be stale.

### `SourceRefreshResult` — new

Returned by `refresh_sources`, combines preview stats with metadata update info:

```rust
pub struct SourceRefreshResult {
    pub preview: PreviewCacheResult,
    pub photos_updated: usize,  // number of PhotoFile entries rewritten
}
```

## Method Signature

```rust
// src/state_manager/refresh_sources.rs
impl StateManager {
    pub fn refresh_sources(&mut self) -> Result<SourceRefreshResult>
}
```

## Logic

```
preview_result = ensure_previews(&self.state, preview_cache_dir)
for photo_id in preview_result.regenerated_ids:
    find photo in self.state.photos (by id)
    re-read EXIF/file metadata from photo.source
    update photo.hash, width_px, height_px, timestamp
return SourceRefreshResult { preview: preview_result, hashes_updated }
```

The hash computation reuses whatever is already used in `commands/add` for consistency.

## File Location

`src/state_manager/refresh_sources.rs` — exposed as a method on `StateManager` via the existing `state_manager.rs` module file (consistent with `page_change_detection.rs` pattern).

## Call Sites

```rust
// before (all three build paths):
let progress = AtomicUsize::new(0);
let preview_cache_dir = mgr.preview_cache_dir();
let cache_result = preview::ensure_previews(&mgr.state, &preview_cache_dir, &progress)?;

// after:
let refresh = mgr.refresh_sources()?;
// use refresh.preview.created, refresh.photos_updated as needed
```

## Tests

- Preview stale → hash, dimensions, timestamp updated; area_weight preserved
- Preview fresh → metadata not recomputed
- Multiple photos, subset stale → only stale entries updated
