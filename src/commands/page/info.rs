//! `fotobuch page info` command.

use std::collections::HashMap;
use std::path::Path;

use crate::dto_models::{PhotoFile, ProjectState};

use super::helpers::{page_idx, resolve_slots};
use super::types::{InfoFilter, PageInfoResult, PageMoveError, SlotInfo, Src};

fn page_dims(state: &ProjectState, idx: usize) -> (bool, f64, f64) {
    let book = &state.config.book;
    if state.has_cover() && idx == 0 {
        let inner = state.layout.len() - 1;
        (true, book.cover.spread_width_mm(inner), book.cover.height_mm)
    } else {
        (false, book.page_width_mm, book.page_height_mm)
    }
}

/// Read metadata for the given address and return per-slot records.
///
/// The `filter` argument is passed through unchanged so callers (CLI handlers)
/// can decide which fields to display without needing a second call.
pub fn execute_info(
    project_root: &Path,
    address: Src,
    _filter: InfoFilter,
) -> Result<PageInfoResult, PageMoveError> {
    use crate::state_manager::StateManager;

    let mgr = StateManager::open(project_root)?;

    let photo_map: HashMap<&str, &PhotoFile> = mgr
        .state
        .photos
        .iter()
        .flat_map(|g| g.files.iter())
        .map(|f| (f.id.as_str(), f))
        .collect();

    let mut slots = Vec::new();

    match &address {
        Src::Pages(pe) => {
            for &p in &pe.pages {
                let idx = page_idx(p, &mgr.state.layout)?;
                let lp = &mgr.state.layout[idx];
                let total = lp.photos.len();
                let (is_cover, page_width_mm, page_height_mm) = page_dims(&mgr.state, idx);
                for (i, photo_id) in lp.photos.iter().enumerate() {
                    if let Some(pf) = photo_map.get(photo_id.as_str()) {
                        slots.push(SlotInfo {
                            page: p,
                            slot: i as u32 + 1,
                            id: pf.id.clone(),
                            source: pf.source.clone(),
                            width_px: pf.width_px,
                            height_px: pf.height_px,
                            area_weight: pf.area_weight,
                            placement: lp.slots.get(i).cloned(),
                            total_page_slots: total,
                            is_cover,
                            page_width_mm,
                            page_height_mm,
                        });
                    }
                }
            }
        }
        Src::Slots { page, slots: slot_expr } => {
            let p = *page;
            let idx = page_idx(p, &mgr.state.layout)?;
            let lp = &mgr.state.layout[idx];
            let total = lp.photos.len();
            let (is_cover, page_width_mm, page_height_mm) = page_dims(&mgr.state, idx);
            let slot_indices = resolve_slots(p, slot_expr, &mgr.state.layout)?;
            for &i in &slot_indices {
                let photo_id = &lp.photos[i];
                if let Some(pf) = photo_map.get(photo_id.as_str()) {
                    slots.push(SlotInfo {
                        page: p,
                        slot: i as u32 + 1,
                        id: pf.id.clone(),
                        source: pf.source.clone(),
                        width_px: pf.width_px,
                        height_px: pf.height_px,
                        area_weight: pf.area_weight,
                        placement: lp.slots.get(i).cloned(),
                        total_page_slots: total,
                        is_cover,
                        page_width_mm,
                        page_height_mm,
                    });
                }
            }
        }
    }

    Ok(PageInfoResult { slots })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::{PagesExpr, SlotExpr};
    use tempfile::TempDir;

    #[test]
    fn test_execute_info_whole_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_info(tmp.path(), Src::Pages(PagesExpr::single(1)), InfoFilter::default()).unwrap();
        assert_eq!(result.slots.len(), 2);
        assert_eq!(result.slots[0].slot, 1);
        assert_eq!(result.slots[0].id, "p0.jpg");
        assert_eq!(result.slots[1].slot, 2);
        assert_eq!(result.slots[0].total_page_slots, 2);
    }

    #[test]
    fn test_execute_info_slot_expr() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_info(
            tmp.path(),
            Src::Slots { page: 1, slots: SlotExpr::from_range(1, 2) },
            InfoFilter::default(),
        ).unwrap();
        assert_eq!(result.slots.len(), 2);
        assert_eq!(result.slots[0].total_page_slots, 3);
    }

    #[test]
    fn test_execute_info_invalid_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let err = execute_info(tmp.path(), Src::Pages(PagesExpr::single(5)), InfoFilter::default()).unwrap_err();
        assert!(err.to_string().contains("page 5"));
    }
}
