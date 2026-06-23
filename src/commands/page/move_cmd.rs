//! `fotobuch page move` command.

mod manual;
mod standard;

use std::path::Path;

use super::swap::execute_swap;
use super::types::{DstMove, PageMoveCmd, PageMoveError, PageMoveResult, Src};
use crate::commands::{CommandOutput, run_write_command};

/// Execute a `page move` command (either Move or Swap variant).
pub fn execute_move(
    project_root: &Path,
    cmd: PageMoveCmd,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    match cmd {
        PageMoveCmd::Move { src, dst } => execute_move_to(project_root, src, dst),
        PageMoveCmd::Swap { left, right } => execute_swap(project_root, left, right),
    }
}

fn execute_move_to(
    project_root: &Path,
    src: Src,
    dst: DstMove,
) -> Result<CommandOutput<PageMoveResult>, PageMoveError> {
    if let DstMove::ManualAt { page, x_mm, y_mm } = dst {
        return manual::execute_move_to_manual(project_root, src, page, x_mm, y_mm);
    }

    // Same-page slot move is a no-op — don't even open the project.
    if let Src::Slots { page, slots: _ } = &src
        && let DstMove::Page(dst_page) = &dst
        && *page == *dst_page
    {
        return Ok(CommandOutput {
            result: empty_result(),
            changed_state: None,
        });
    }

    run_write_command(project_root, |mgr| {
        let mut view = mgr.get_write_layout_state();
        standard::apply_move(&mut view, src, dst)
    })
}

pub(super) fn empty_result() -> PageMoveResult {
    PageMoveResult {
        pages_modified: vec![],
        pages_inserted: vec![],
        pages_deleted: vec![],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::super::types::{DstMove, PageMoveCmd, PagesExpr, SlotExpr, Src, ValidationError};
    use super::*;
    use crate::state_manager::StateManager;
    use tempfile::TempDir;

    #[test]
    fn test_execute_move_pages_to_page() {
        let state =
            make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg", "p3.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(1)),
            dst: DstMove::Page(0),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.result.pages_deleted.contains(&1));

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        let page1 = &mgr.state().layout[0];
        assert!(page1.photos.contains(&"p2.jpg".to_owned()));
        assert!(page1.photos.contains(&"p3.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_unplace_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(0)),
            dst: DstMove::Unplace,
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.result.pages_deleted, vec![0]);
        assert!(result.result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        assert_eq!(mgr.state().layout[0].photos, vec!["p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_unplace_slots() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            dst: DstMove::Unplace,
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.result.pages_modified, vec![0]);
        assert!(result.result.pages_deleted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout[0].photos, vec!["p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_to_new_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 3);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_slots_to_new_page_after() {
        // Regression: "page move 2:1 to 1+" must not fail with SlotNotFound.
        // Inserting the new page after page 1 shifts page 2 from index 1 to index 2;
        // slot resolution must happen before the insert.
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 1,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        // Original page 0, new page (with p1.jpg), original page 1 (with p2.jpg)
        assert_eq!(mgr.state().layout.len(), 3);
        assert_eq!(mgr.state().layout[0].photos, vec!["p0.jpg"]);
        assert!(mgr.state().layout[1].photos.contains(&"p1.jpg".to_owned()));
        assert!(mgr.state().layout[2].photos.contains(&"p2.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_slots_to_page_deletes_empty_src() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg", "p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Move the only slot from page 0 to page 1 → page 0 becomes empty → deleted
        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::Page(1),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.result.pages_deleted.contains(&0));

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        assert!(mgr.state().layout[0].photos.contains(&"p0.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_move_unplace_all_slots_deletes_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Unplace all slots from page 0 → page 0 becomes empty → deleted
        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            dst: DstMove::Unplace,
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(result.result.pages_deleted.contains(&0));
        assert!(result.result.pages_modified.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 1);
        assert_eq!(mgr.state().layout[0].photos, vec!["p2.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_page_range_block_transposition() {
        let state = make_state_with_layout(vec![
            vec!["a1.jpg", "a2.jpg"],           // page 1
            vec!["b1.jpg"],                     // page 2
            vec!["c1.jpg", "c2.jpg", "c3.jpg"], // page 3
            vec!["d1.jpg"],                     // page 4
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Equal-size block transposition: [0,1] ↔ [2,3]
        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_range(0, 1)),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_range(2, 3)),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.result.pages_modified, vec![0, 1, 2, 3]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(
            mgr.state().layout[0].photos,
            vec!["c1.jpg", "c2.jpg", "c3.jpg"]
        );
        assert_eq!(mgr.state().layout[1].photos, vec!["d1.jpg"]);
        assert_eq!(mgr.state().layout[2].photos, vec!["a1.jpg", "a2.jpg"]);
        assert_eq!(mgr.state().layout[3].photos, vec!["b1.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_page_range_unequal_sizes_with_middle() {
        // [0,1] ↔ [3,4,5] with page 2 as middle
        // before: [a, b, M, c, d, e]
        // after:  [c, d, e, M, a, b]
        let state = make_state_with_layout(vec![
            vec!["a.jpg"],
            vec!["b.jpg"],
            vec!["M.jpg"],
            vec!["c.jpg"],
            vec!["d.jpg"],
            vec!["e.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_range(0, 1)),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_range(3, 5)),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert_eq!(result.result.pages_modified, vec![0, 1, 3, 4, 5]);

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout[0].photos, vec!["c.jpg"]);
        assert_eq!(mgr.state().layout[1].photos, vec!["d.jpg"]);
        assert_eq!(mgr.state().layout[2].photos, vec!["e.jpg"]);
        assert_eq!(mgr.state().layout[3].photos, vec!["M.jpg"]);
        assert_eq!(mgr.state().layout[4].photos, vec!["a.jpg"]);
        assert_eq!(mgr.state().layout[5].photos, vec!["b.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_page_range_non_contiguous_error() {
        let state = make_state_with_layout(vec![
            vec!["a.jpg"],
            vec!["b.jpg"],
            vec!["c.jpg"],
            vec!["d.jpg"],
        ]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_list(vec![0, 2])),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_list(vec![1, 3])),
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::SwapNonContiguous)
        ));
    }

    #[test]
    fn test_execute_swap_page_range_overlap() {
        let state = make_state_with_layout(vec![vec!["a.jpg"], vec!["b.jpg"], vec!["c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Pages(PagesExpr::from_range(0, 1)),
            right: super::super::types::DstSwap::Pages(PagesExpr::from_range(1, 2)),
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::SwapRangesOverlap)
        ));
    }

    #[test]
    fn test_execute_swap_same_page_slots_allowed() {
        // swap 0:0 0:2 — slot 0 and slot 2 on the same page swap positions.
        let state = make_state_with_layout(vec![vec!["a.jpg", "b.jpg", "c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            right: super::super::types::DstSwap::Slots {
                page: 0,
                slots: SlotExpr::single(2),
            },
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(
            mgr.state().layout[0].photos,
            vec!["c.jpg", "b.jpg", "a.jpg"]
        );
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_execute_swap_same_page_slots_overlap_error() {
        let state = make_state_with_layout(vec![vec!["a.jpg", "b.jpg", "c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Swap {
            left: Src::Slots {
                page: 0,
                slots: SlotExpr::from_range(0, 1),
            },
            right: super::super::types::DstSwap::Slots {
                page: 0,
                slots: SlotExpr::from_range(1, 2),
            },
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::SwapRangesOverlap)
        ));
    }

    #[test]
    fn execute_move_new_page_at_zero_inserts_before_first_page() {
        // Page 1 has two photos so it won't be deleted after the move.
        let state = make_state_with_layout(vec![vec!["a.jpg"], vec!["b.jpg", "c.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 1,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(0),
        };
        let result = execute_move(tmp.path(), cmd).unwrap();
        assert!(!result.result.pages_inserted.is_empty());

        let mgr = StateManager::open(tmp.path()).unwrap();
        // new page(b.jpg), page0(a.jpg), page1(c.jpg)
        assert_eq!(mgr.state().layout.len(), 3);
        assert!(mgr.state().layout[0].photos.contains(&"b.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn execute_move_new_page_at_len_appends() {
        let state = make_state_with_layout(vec![vec!["a.jpg", "b.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(1),
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(mgr.state().layout.len(), 2);
        assert!(mgr.state().layout[1].photos.contains(&"a.jpg".to_owned()));
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn move_slot_into_manual_page_creates_positioned_slot() {
        use super::super::mode::execute_mode;
        use crate::models::PageMode;

        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);
        // Page 1 becomes Manual.
        execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::ManualAt {
                page: 1,
                x_mm: 50.0,
                y_mm: 60.0,
            },
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        let dst = &mgr.state().layout[1];
        // Manual page now holds both photos with matching slot counts.
        assert_eq!(dst.photos, vec!["p2.jpg", "p0.jpg"]);
        assert_eq!(dst.slots.len(), dst.photos.len());
        // New slot keeps the source slot size (fixture: 100 x 80) at the drop point.
        let new = dst.slots.last().unwrap();
        assert_eq!((new.x_mm, new.y_mm), (50.0, 60.0));
        assert_eq!((new.width_mm, new.height_mm), (100.0, 80.0));
        // Source page lost the moved photo.
        assert_eq!(mgr.state().layout[0].photos, vec!["p1.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn move_into_non_manual_page_is_rejected() {
        let state = make_state_with_layout(vec![vec!["p0.jpg", "p1.jpg"], vec!["p2.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::ManualAt {
                page: 1,
                x_mm: 0.0,
                y_mm: 0.0,
            },
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotManual(1))
        ));
    }

    #[test]
    fn swap_into_manual_page_adapts_slot_height_to_photo_ratio() {
        use super::super::mode::execute_mode;
        use super::super::types::DstSwap;
        use crate::models::PageMode;

        // Fixture photos are 4000 x 3000 (4:3).
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec!["p1.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);
        execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        let cmd = PageMoveCmd::Swap {
            left: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            right: DstSwap::Slots {
                page: 1,
                slots: SlotExpr::single(0),
            },
        };
        execute_move(tmp.path(), cmd).unwrap();

        let mgr = StateManager::open(tmp.path()).unwrap();
        let manual_slot = &mgr.state().layout[1].slots[0];
        // Width kept (100), height now matches the incoming photo's ratio: 100 * 3/4 = 75.
        assert_eq!(manual_slot.width_mm, 100.0);
        assert!((manual_slot.height_mm - 75.0).abs() < 1e-9);
        assert_eq!(mgr.state().layout[1].photos, vec!["p0.jpg"]);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn execute_move_new_page_at_out_of_range_errors() {
        let state = make_state_with_layout(vec![vec!["a.jpg"]]);
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Slots {
                page: 0,
                slots: SlotExpr::single(0),
            },
            dst: DstMove::NewPageAt(99),
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotFound(99))
        ));
    }

    #[test]
    fn execute_move_pages_unplace_rejects_cover_when_active() {
        use crate::models::{BookConfig, CoverConfig, ProjectConfig};
        let mut state = make_state_with_layout(vec![vec!["cover.jpg"], vec!["a.jpg"]]);
        state.config = ProjectConfig {
            book: BookConfig {
                cover: CoverConfig {
                    active: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let tmp = TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let cmd = PageMoveCmd::Move {
            src: Src::Pages(PagesExpr::single(0)),
            dst: DstMove::Unplace,
        };
        let err = execute_move(tmp.path(), cmd).unwrap_err();
        assert!(matches!(
            err,
            PageMoveError::Validation(ValidationError::PageNotFound(0))
        ));
    }
}
