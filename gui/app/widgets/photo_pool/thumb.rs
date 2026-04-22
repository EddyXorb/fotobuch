use std::collections::HashSet;
use std::path::PathBuf;

use crate::state::GuiState;
use crate::task::BackgroundTask;

const THUMB_FILL_CHUNK: usize = 8;

/// Dispatches thumbnail loading tasks: visible items first, then prefetch.
pub fn dispatch_thumb_loads(state: &mut GuiState, visible_needed: Vec<String>) {
    let mut visible_filtered: Vec<(String, PathBuf)> = visible_needed
        .into_iter()
        .filter(|id| !state.thumb.thumbs.contains_key(id) && !state.thumb.in_flight.contains(id))
        .filter_map(|id| {
            state
                .derived
                .photo_by_id
                .get(&id)
                .map(|f| (id, PathBuf::from(&f.source)))
        })
        .collect();

    let mut seen = HashSet::new();
    visible_filtered.retain(|(id, _)| seen.insert(id.clone()));

    if !visible_filtered.is_empty() {
        for (id, _) in &visible_filtered {
            state.thumb.in_flight.insert(id.clone());
        }
        state.thumb.pending_loads.extend(visible_filtered);
    } else if !state.thumb.prefetch.is_empty() {
        let chunk_len = THUMB_FILL_CHUNK.min(state.thumb.prefetch.len());
        let chunk: Vec<String> = state.thumb.prefetch.drain(..chunk_len).collect();
        let items: Vec<(String, PathBuf)> = chunk
            .into_iter()
            .filter(|id| {
                !state.thumb.thumbs.contains_key(id) && !state.thumb.in_flight.contains(id)
            })
            .filter_map(|id| {
                state
                    .derived
                    .photo_by_id
                    .get(&id)
                    .map(|f| (id, PathBuf::from(&f.source)))
            })
            .collect();
        for (id, _) in &items {
            state.thumb.in_flight.insert(id.clone());
        }
        state.thumb.pending_loads.extend(items);
    }
}

/// Flushes pending thumb loads as a single BackgroundTask.
pub fn flush_thumb_loads(state: &mut GuiState) -> Option<BackgroundTask> {
    if state.thumb.pending_loads.is_empty() {
        return None;
    }
    let items: Vec<(String, PathBuf)> = state.thumb.pending_loads.drain(..).collect();
    Some(BackgroundTask::LoadPhotoThumbnails { items })
}
