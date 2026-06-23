//! `fotobuch page weight` command.

use std::path::Path;

use crate::commands::{CommandOutput, run_write_command};
use crate::state_manager::{ReadOnlyState, WritePhotosState};

use super::helpers::{page_idx, resolve_slots};
use super::types::{PageMoveError, ValidationError, WeightAddress};

/// Set `area_weight` on the photos at the given address.
pub fn execute_weight(
    project_root: &Path,
    address: WeightAddress,
    weight: f64,
) -> Result<CommandOutput<()>, PageMoveError> {
    if weight <= 0.0 {
        return Err(ValidationError::WeightOutOfRange(weight).into());
    }

    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_photos_state();
        let page = apply_weight(&mut view, &address, weight)?;

        Ok((format!("page weight: page {page} = {weight}"), ()))
    })
}

/// Resolve the addressed photos from the layout and set their `area_weight`.
/// Returns the addressed page number (for the commit message).
fn apply_weight(
    s: &mut WritePhotosState,
    address: &WeightAddress,
    weight: f64,
) -> Result<u32, ValidationError> {
    let (page, slot_indices): (u32, Vec<usize>) = match address {
        WeightAddress::Page(p) => {
            let idx = page_idx(*p, s.layout())?;
            let n = s.layout()[idx].photos.len();
            (*p, (0..n).collect())
        }
        WeightAddress::Slots { page, slots } => {
            let indices = resolve_slots(*page, slots, s.layout())?;
            (*page, indices)
        }
    };

    let page_idx_val = page_idx(page, s.layout())?;
    let photo_ids: Vec<String> = slot_indices
        .iter()
        .map(|&i| s.layout()[page_idx_val].photos[i].clone())
        .collect();

    for photo_id in &photo_ids {
        for group in s.photos_mut() {
            for file in &mut group.files {
                if file.id == *photo_id {
                    file.area_weight = weight;
                }
            }
        }
    }

    Ok(page)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::{SlotExpr, ValidationError};
    use super::*;
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_weight_whole_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        execute_weight(tmp.path(), WeightAddress::Page(0), 2.5).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        let files = &mgr.state.photos[0].files;
        let p0 = files.iter().find(|f| f.id == "p0.jpg").unwrap();
        let p1 = files.iter().find(|f| f.id == "p1.jpg").unwrap();
        assert_eq!(p0.area_weight, 2.5);
        assert_eq!(p1.area_weight, 2.5);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_weight_specific_slots() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        execute_weight(
            tmp.path(),
            WeightAddress::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            3.0,
        )
        .unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        let files = &mgr.state.photos[0].files;
        assert_eq!(
            files.iter().find(|f| f.id == "p0.jpg").unwrap().area_weight,
            3.0
        );
        assert_eq!(
            files.iter().find(|f| f.id == "p1.jpg").unwrap().area_weight,
            3.0
        );
        assert_eq!(
            files.iter().find(|f| f.id == "p2.jpg").unwrap().area_weight,
            1.0
        );
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_weight_zero_is_error() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let err = execute_weight(tmp.path(), WeightAddress::Page(0), 0.0).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::WeightOutOfRange(_))
        ));
    }

    #[test]
    fn test_execute_weight_invalid_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let err = execute_weight(tmp.path(), WeightAddress::Page(99), 1.5).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotFound(99))
        ));
    }
}
