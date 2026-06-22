//! `fotobuch unplace` command — remove photos from specific slots.

use std::path::Path;

use crate::commands::page::{PageMoveError, SlotExpr, apply_unplace};
use crate::commands::{CommandOutput, run_write_command};

/// Result of unplacing photos.
#[derive(Debug)]
pub struct UnplaceResult {
    pub pages_modified: Vec<u32>,
    pub pages_inserted: Vec<u32>,
    pub pages_deleted: Vec<u32>,
}

/// Remove photos from the layout at the given page:slot address.
///
/// Photos are kept in `state.photos` (they become "unplaced").
pub fn execute_unplace(
    project_root: &Path,
    page: u32,
    slots: SlotExpr,
) -> Result<CommandOutput<UnplaceResult>, PageMoveError> {
    run_write_command(project_root, |mgr| {
        let (deleted, modified) = apply_unplace(&mut mgr.state.layout, page, &slots)?;
        let commit_msg = if deleted.is_empty() && modified.is_empty() {
            String::new()
        } else {
            format!("unplace: page {page}")
        };
        Ok((
            commit_msg,
            UnplaceResult {
                pages_modified: modified,
                pages_inserted: vec![],
                pages_deleted: deleted,
            },
        ))
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
        assert_eq!(result.result.pages_modified, vec![0]);

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
        assert!(result.result.pages_deleted.contains(&0));
        assert!(result.result.pages_modified.is_empty());

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
