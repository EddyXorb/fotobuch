//! `page pos` — Freie Slot-Positionierung im Manual-Mode.

use std::path::Path;

use crate::{
    dto_models::{LayoutPage, PageMode, Slot},
    state_manager::StateManager,
};

use super::{
    helpers::{page_idx, resolve_slots},
    types::{PageMoveError, SlotExpr, ValidationError},
};

// ── Public types ──────────────────────────────────────────────────────────────

/// How to change the position of a slot.
#[derive(Debug, Clone, PartialEq)]
pub enum PosMode {
    /// Move relative to current position.
    Relative { dx_mm: f64, dy_mm: f64 },
    /// Move to an absolute position.
    Absolute { x_mm: f64, y_mm: f64 },
}

/// Configuration for a `page pos` call.
///
/// At least one of `position` or `scale` must be `Some`.
#[derive(Debug, Clone, PartialEq)]
pub struct PosConfig {
    /// Position change — `None` when only `--scale` is given.
    pub position: Option<PosMode>,
    /// Scale factor applied to `width_mm` and `height_mm`. `None` = no scaling.
    pub scale: Option<f64>,
}

/// One slot's before/after state.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotChange {
    pub slot: usize,
    pub old: Slot,
    pub new: Slot,
}

/// Result of a successful `page pos` call.
#[derive(Debug, Clone)]
pub struct PosResult {
    pub page: u32,
    pub slots_changed: Vec<SlotChange>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_in_manual_mode(layout: &[LayoutPage], idx: usize) -> bool {
    layout[idx].mode == Some(PageMode::Manual)
}

// ── execute_pos ───────────────────────────────────────────────────────────────

/// Reposition one or more slots on a Manual-mode page.
///
/// # Errors
/// - [`ValidationError::PageNotFound`] if `page` does not exist.
/// - [`ValidationError::PageNotManual`] if the page is not in Manual mode.
/// - [`ValidationError::SlotNotFound`] if any addressed slot index is out of range.
pub fn execute_pos(
    project_root: &Path,
    page: u32,
    slots: SlotExpr,
    config: &PosConfig,
) -> Result<PosResult, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    let idx = page_idx(page, &mgr.state.layout)?;

    // Require Manual mode
    if !is_in_manual_mode(&mgr.state.layout, idx) {
        return Err(ValidationError::PageNotManual(page).into());
    }

    let slot_indices = resolve_slots(page, &slots, &mgr.state.layout)?;

    let mut slots_changed = Vec::new();

    for slot_idx in slot_indices {
        let old = mgr.state.layout[idx].slots[slot_idx].clone();
        let mut new = old.clone();

        // Apply position change
        match &config.position {
            Some(PosMode::Relative { dx_mm, dy_mm }) => {
                new.x_mm += dx_mm;
                new.y_mm += dy_mm;
            }
            Some(PosMode::Absolute { x_mm, y_mm }) => {
                new.x_mm = *x_mm;
                new.y_mm = *y_mm;
            }
            None => {}
        }

        // Apply scale (origin stays, width/height grow right-downward)
        if let Some(s) = config.scale {
            new.width_mm *= s;
            new.height_mm *= s;
        }

        mgr.state.layout[idx].slots[slot_idx] = new.clone();
        slots_changed.push(SlotChange {
            slot: slot_idx,
            old,
            new,
        });
    }

    mgr.finish(&format!(
        "page pos {page}: {} slot(s) moved",
        slots_changed.len()
    ))?;

    Ok(PosResult {
        page,
        slots_changed,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::SlotExpr;
    use super::*;
    use crate::{
        commands::page::mode::execute_mode, dto_models::PageMode, state_manager::StateManager,
    };

    fn make_manual_state() -> (ProjectState, tempfile::TempDir) {
        use crate::commands::page::types::PagesExpr;
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);
        // Set page 0 to Manual
        execute_mode(tmp.path(), PagesExpr::single(0), PageMode::Manual).unwrap();
        (state, tmp)
    }

    use crate::dto_models::ProjectState;

    #[test]
    fn test_relative_move() {
        let (_state, tmp) = make_manual_state();

        // Slot 0 starts at (0,0); move by (+5, -3)
        let config = PosConfig {
            position: Some(PosMode::Relative {
                dx_mm: 5.0,
                dy_mm: -3.0,
            }),
            scale: None,
        };
        let result = execute_pos(tmp.path(), 0, SlotExpr::single(0), &config).unwrap();

        assert_eq!(result.slots_changed.len(), 1);
        let change = &result.slots_changed[0];
        assert_eq!(change.old.x_mm, 0.0);
        assert_eq!(change.old.y_mm, 0.0);
        assert_eq!(change.new.x_mm, 5.0);
        assert_eq!(change.new.y_mm, -3.0);

        // Verify persisted
        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state.layout[0].slots[0].x_mm, 5.0);
        assert_eq!(mgr.state.layout[0].slots[0].y_mm, -3.0);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_absolute_move() {
        let (_state, tmp) = make_manual_state();

        let config = PosConfig {
            position: Some(PosMode::Absolute {
                x_mm: 50.0,
                y_mm: 60.0,
            }),
            scale: None,
        };
        let result = execute_pos(tmp.path(), 0, SlotExpr::single(0), &config).unwrap();

        let change = &result.slots_changed[0];
        assert_eq!(change.new.x_mm, 50.0);
        assert_eq!(change.new.y_mm, 60.0);
    }

    #[test]
    fn test_scale_only() {
        let (_state, tmp) = make_manual_state();

        let config = PosConfig {
            position: None,
            scale: Some(2.0),
        };
        let result = execute_pos(tmp.path(), 0, SlotExpr::single(0), &config).unwrap();

        let change = &result.slots_changed[0];
        // Origin unchanged
        assert_eq!(change.new.x_mm, change.old.x_mm);
        assert_eq!(change.new.y_mm, change.old.y_mm);
        // Width/height doubled (fixture: 100 x 80)
        assert_eq!(change.new.width_mm, 200.0);
        assert_eq!(change.new.height_mm, 160.0);
    }

    #[test]
    fn test_relative_and_scale() {
        let (_state, tmp) = make_manual_state();

        let config = PosConfig {
            position: Some(PosMode::Relative {
                dx_mm: 5.0,
                dy_mm: 5.0,
            }),
            scale: Some(0.5),
        };
        let result = execute_pos(tmp.path(), 0, SlotExpr::single(0), &config).unwrap();

        let change = &result.slots_changed[0];
        assert_eq!(change.new.x_mm, 5.0);
        assert_eq!(change.new.y_mm, 5.0);
        assert_eq!(change.new.width_mm, 50.0);
        assert_eq!(change.new.height_mm, 40.0);
    }

    #[test]
    fn test_multi_slot_range() {
        let (_state, tmp) = make_manual_state();

        let config = PosConfig {
            position: Some(PosMode::Relative {
                dx_mm: 10.0,
                dy_mm: 10.0,
            }),
            scale: None,
        };
        let result = execute_pos(tmp.path(), 0, SlotExpr::from_range(0, 2), &config).unwrap();

        assert_eq!(result.slots_changed.len(), 3);
        for change in &result.slots_changed {
            assert_eq!(change.new.x_mm, 10.0);
            assert_eq!(change.new.y_mm, 10.0);
        }
    }

    #[test]
    fn test_error_not_manual() {
        // Page in Auto mode → should fail
        let state = make_state_with_layout(vec![vec!["p0.jpg"]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let config = PosConfig {
            position: Some(PosMode::Relative {
                dx_mm: 1.0,
                dy_mm: 0.0,
            }),
            scale: None,
        };
        let err = execute_pos(tmp.path(), 0, SlotExpr::single(0), &config).unwrap_err();
        match err {
            PageMoveError::Validation(ValidationError::PageNotManual(p)) => assert_eq!(p, 0),
            other => panic!("expected PageNotManual, got {other:?}"),
        }
    }

    #[test]
    fn test_error_slot_out_of_range() {
        let (_state, tmp) = make_manual_state();

        let config = PosConfig {
            position: Some(PosMode::Relative {
                dx_mm: 1.0,
                dy_mm: 0.0,
            }),
            scale: None,
        };
        // Page 0 has 3 slots (0..2); slot 99 does not exist
        let err = execute_pos(tmp.path(), 0, SlotExpr::single(99), &config).unwrap_err();
        match err {
            PageMoveError::Validation(ValidationError::SlotNotFound { page, slot }) => {
                assert_eq!(page, 0);
                assert_eq!(slot, 99);
            }
            other => panic!("expected SlotNotFound, got {other:?}"),
        }
    }
}
