//! `fotobuch page split` command.

use std::path::Path;

use crate::commands::{CommandOutput, run_write_command};
use crate::models::{LayoutPage, PageMode};
use crate::state_manager::{ReadOnlyState, WriteLayoutState};

use super::helpers::page_idx;
use super::types::{PageMoveError, PageMoveResult, ValidationError};

/// Split a page at a given slot: photos from `slot` onwards move to a new page after it.
///
/// `page` and `slot` are 0-based.
pub fn execute_split(
    project_root: &Path,
    page: u32,
    slot: u32,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_layout_state();
        let new_page_num = split_page_at_slot(&mut view, page, slot)?;

        Ok((
            format!("page split: page {page} at slot {slot}"),
            PageMoveResult {
                pages_modified: vec![page],
                pages_inserted: vec![new_page_num],
                pages_deleted: vec![],
            },
        ))
    })
}

/// Move photos/slots from `slot` onwards onto a fresh page after `page`.
/// Returns the 0-based index of the inserted page.
fn split_page_at_slot(
    s: &mut WriteLayoutState,
    page: u32,
    slot: u32,
) -> Result<u32, ValidationError> {
    let idx = page_idx(page, s.layout())?;
    let n_photos = s.layout()[idx].photos.len();

    if slot as usize >= n_photos {
        return Err(ValidationError::SlotNotFound { page, slot });
    }
    if slot == 0 {
        return Err(ValidationError::SplitAtFirstSlot(page));
    }

    let split_at = slot as usize;
    let new_idx = idx + 1;
    let layout = s.layout_mut();
    let moved_photos: Vec<String> = layout[idx].photos[split_at..].to_vec();
    let moved_slots: Vec<_> = if split_at < layout[idx].slots.len() {
        layout[idx].slots[split_at..].to_vec()
    } else {
        vec![]
    };
    layout[idx].photos.truncate(split_at);
    layout[idx].slots.truncate(split_at);
    layout.insert(
        new_idx,
        LayoutPage {
            photos: moved_photos,
            slots: moved_slots,
            mode: PageMode::Auto,
        },
    );

    Ok(new_idx as u32)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::ValidationError;
    use super::*;
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_split_creates_new_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg", "p3.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_split(tmp.path(), 0, 2).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

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

        let result = execute_split(tmp.path(), 0, 0);
        assert!(matches!(
            result,
            Err(PageMoveError::Validation(
                ValidationError::SplitAtFirstSlot(0)
            ))
        ));
    }
}
