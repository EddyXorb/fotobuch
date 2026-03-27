//! `fotobuch unplace` command — remove photos from specific slots.

use std::path::Path;

use crate::state_manager::StateManager;

use crate::commands::page::{
    PageMoveError, PageMoveResult, SlotExpr, delete_empty_pages, page_idx, remove_slots,
    resolve_slots,
};

/// Remove photos from the layout at the given page:slot address.
///
/// Photos are kept in `state.photos` (they become "unplaced").
/// Returns the 0-based page numbers that were modified.
pub fn execute_unplace(
    project_root: &Path,
    page: u32,
    slots: SlotExpr,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    let slot_indices = resolve_slots(page, &slots, &mgr.state.layout)?;
    if slot_indices.is_empty() {
        return Ok(PageMoveResult {
            pages_modified: vec![],
            pages_inserted: vec![],
            pages_deleted: vec![],
        });
    }

    let page_idx_val = page_idx(page, &mgr.state.layout)?;
    remove_slots(&mut mgr.state.layout, page_idx_val, slot_indices);
    let deleted = delete_empty_pages(&mut mgr.state.layout);
    let modified = if deleted.contains(&page) {
        vec![]
    } else {
        vec![page]
    };

    mgr.finish(&format!("unplace: page {page}"))?;

    Ok(PageMoveResult {
        pages_modified: modified,
        pages_inserted: vec![],
        pages_deleted: deleted,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::page::test_fixtures::{make_state_with_layout, setup_repo};
    use crate::commands::page::{SlotExpr, ValidationError};
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_unplace_removes_photo() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 0, SlotExpr::single(1)).unwrap();
        assert_eq!(result.pages_modified, vec![0]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        let page = &mgr.state.layout[0];
        assert_eq!(page.photos, vec!["p0.jpg", "p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_unplace_last_photo_deletes_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 0, SlotExpr::single(0)).unwrap();
        assert!(result.pages_deleted.contains(&0));
        assert!(result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 1);
        assert_eq!(mgr.state.layout[0].photos, vec!["p1.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_unplace_invalid_slot() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 0, SlotExpr::single(5));
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::SlotNotFound {
                page: 0,
                slot: 5
            }))
        ));
    }

    #[test]
    fn test_execute_unplace_invalid_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_unplace(tmp.path(), 99, SlotExpr::single(1));
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::PageNotFound(99)))
        ));
    }
}
