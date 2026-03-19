//! `fotobuch page split` command.

use std::path::Path;

use crate::dto_models::LayoutPage;
use crate::state_manager::StateManager;

use super::helpers::page_idx;
use super::types::{PageMoveError, PageMoveResult, ValidationError};

/// Split a page at a given slot: photos from `slot` onwards move to a new page after it.
///
/// `page` and `slot` are 1-based.
pub fn execute_split(
    project_root: &Path,
    page: u32,
    slot: u32,
) -> Result<PageMoveResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    let idx = page_idx(page, &mgr.state.layout)?;
    let n_photos = mgr.state.layout[idx].photos.len();

    if slot == 0 || slot as usize > n_photos {
        return Err(ValidationError::SlotNotFound { page, slot }.into());
    }
    if slot == 1 {
        return Err(ValidationError::SplitAtFirstSlot(page).into());
    }

    let split_at = slot as usize - 1;
    let moved_photos: Vec<String> = mgr.state.layout[idx].photos[split_at..].to_vec();
    let moved_slots: Vec<_> = if split_at < mgr.state.layout[idx].slots.len() {
        mgr.state.layout[idx].slots[split_at..].to_vec()
    } else {
        vec![]
    };

    mgr.state.layout[idx].photos.truncate(split_at);
    mgr.state.layout[idx].slots.truncate(split_at);

    let new_idx = idx + 1;
    mgr.state.layout.insert(
        new_idx,
        LayoutPage {
            page: (new_idx + 1) as usize, // will be renumbered by finish()
            photos: moved_photos,
            slots: moved_slots,
        },
    );

    let new_page_num = new_idx as u32 + 1;
    mgr.finish(&format!("page split: page {page} at slot {slot}"))?;

    Ok(PageMoveResult {
        pages_modified: vec![page],
        pages_inserted: vec![new_page_num],
        pages_deleted: vec![],
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::ValidationError;
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_split_creates_new_page() {
        let state =
            make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg", "p3.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_split(tmp.path(), 1, 3).unwrap();
        assert!(!result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout.len(), 2);
        assert_eq!(mgr.state.layout[0].photos, vec!["p0.jpg", "p1.jpg"]);
        assert_eq!(mgr.state.layout[1].photos, vec!["p2.jpg", "p3.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_split_at_first_slot_is_error() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_split(tmp.path(), 1, 1);
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(ValidationError::SplitAtFirstSlot(1)))
        ));
    }
}
