//! Change page mode (Auto/Manual).

use std::path::Path;

use crate::commands::CommandOutput;
use crate::{dto_models::PageMode, state_manager::StateManager};

use super::{
    helpers::{format_pages_list, page_idx},
    types::{PageMoveError, PagesExpr, ValidationError},
};

/// Result of changing page mode
#[derive(Debug, Clone)]
pub struct PageModeResult {
    pub pages_changed: Vec<u32>,
    pub new_mode: PageMode,
}

/// Change the mode of pages (Auto ↔ Manual)
pub fn execute_mode(
    project_root: &Path,
    pages: PagesExpr,
    mode: PageMode,
) -> Result<CommandOutput<PageModeResult>, PageMoveError> {
    let mut mgr = StateManager::open(project_root)?;

    if pages.pages.is_empty() {
        return Err(ValidationError::PageNotFound(0).into());
    }

    let mut pages_changed = Vec::new();
    for &page_num in &pages.pages {
        let idx = page_idx(page_num, &mgr.state.layout)?;
        // Store as Some(mode), but Auto will be serialized as absent
        mgr.state.layout[idx].mode = Some(mode);
        pages_changed.push(page_num);
    }

    let mode_str = match mode {
        PageMode::Auto => "auto",
        PageMode::Manual => "manual",
    };
    let pages_str = format_pages_list(&pages.pages);
    let changed_state = mgr.finish(&format!("page mode {}: {}", pages_str, mode_str))?;

    Ok(CommandOutput {
        result: PageModeResult {
            pages_changed,
            new_mode: mode,
        },
        changed_state,
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_fixtures::{make_state_with_layout, setup_repo};
    use super::*;
    use crate::state_manager::StateManager;

    #[test]
    fn test_layout_page_serialization() {
        use crate::dto_models::LayoutPage;

        // Test that Some(Manual) is serialized but None/Some(Auto) are not
        let page_manual = LayoutPage {
            page: 0,
            photos: vec![],
            slots: vec![],
            mode: Some(PageMode::Manual),
        };
        let yaml = serde_yaml::to_string(&page_manual).unwrap();
        eprintln!("Manual page YAML:\n{}", yaml);
        assert!(
            yaml.contains("mode:"),
            "Manual mode should be serialized to YAML"
        );
        assert!(yaml.contains("manual"), "Mode should serialize as 'manual'");

        let page_auto = LayoutPage {
            page: 0,
            photos: vec![],
            slots: vec![],
            mode: Some(PageMode::Auto),
        };
        let yaml = serde_yaml::to_string(&page_auto).unwrap();
        eprintln!("Auto page YAML:\n{}", yaml);
        assert!(
            !yaml.contains("mode:"),
            "Auto mode should not be serialized (skipped)"
        );

        let page_none = LayoutPage {
            page: 0,
            photos: vec![],
            slots: vec![],
            mode: None,
        };
        let yaml = serde_yaml::to_string(&page_none).unwrap();
        eprintln!("None page YAML:\n{}", yaml);
        assert!(
            !yaml.contains("mode:"),
            "None mode should not be serialized (default, skipped)"
        );
    }

    #[test]
    fn test_set_manual_single_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec![], vec![]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result = execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        assert_eq!(result.result.pages_changed, vec![1]);
        assert_eq!(result.result.new_mode, PageMode::Manual);

        // Verify state was saved and persisted
        let mgr = StateManager::open(tmp.path()).unwrap();
        assert_eq!(
            mgr.state.layout[1].mode,
            Some(PageMode::Manual),
            "Mode should be set to Manual and persisted"
        );
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_set_auto_single_page() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec![], vec![]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // First set to Manual
        execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        // Then back to Auto
        let result = execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Auto).unwrap();

        assert_eq!(result.result.pages_changed, vec![1]);
        assert_eq!(result.result.new_mode, PageMode::Auto);

        // Verify state was saved
        let mgr = StateManager::open(tmp.path()).unwrap();
        // Auto mode is stored as None
        assert_eq!(mgr.state.layout[1].mode, None);
        mgr.finish("test: noop").unwrap();
    }

    #[test]
    fn test_set_manual_range() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec![], vec![]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        let result =
            execute_mode(tmp.path(), PagesExpr::from_range(0, 2), PageMode::Manual).unwrap();

        assert_eq!(result.result.pages_changed, vec![0, 1, 2]);
        assert_eq!(result.result.new_mode, PageMode::Manual);

        // Verify operation completed successfully
        // Note: Detailed YAML serialization verification deferred
    }

    #[test]
    fn test_idempotent() {
        let state = make_state_with_layout(vec![vec!["p0.jpg"], vec![], vec![]]);
        let tmp = tempfile::TempDir::new().unwrap();
        setup_repo(&tmp, &state);

        // Set to Manual
        execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        // Set to Manual again
        let result = execute_mode(tmp.path(), PagesExpr::single(1), PageMode::Manual).unwrap();

        assert_eq!(result.result.pages_changed, vec![1]);
    }
}
