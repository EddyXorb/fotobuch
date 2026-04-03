//! Shared test fixtures for page command tests.

use crate::dto_models::{
    BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup, ProjectConfig,
    ProjectState, Slot,
};
use crate::git;
use tempfile::TempDir;

pub fn make_slot() -> Slot {
    Slot {
        x_mm: 0.0,
        y_mm: 0.0,
        width_mm: 100.0,
        height_mm: 80.0,
    }
}

pub fn make_state_with_layout(pages: Vec<Vec<&str>>) -> ProjectState {
    let layout: Vec<LayoutPage> = pages
        .into_iter()
        .enumerate()
        .map(|(i, photos)| LayoutPage {
            page: i,
            photos: photos.iter().map(|s| s.to_string()).collect(),
            slots: (0..photos.len()).map(|_| make_slot()).collect(),
            mode: None, // Auto pages have None
        })
        .collect();

    ProjectState {
        config: ProjectConfig {
            book: BookConfig {
                title: "Test".to_owned(),
                page_width_mm: 420.0,
                page_height_mm: 297.0,
                bleed_mm: 3.0,
                margin_mm: 10.0,
                gap_mm: 5.0,
                bleed_threshold_mm: 3.0,
                dpi: 300.0,
                cover: Default::default(),
                appendix: Default::default(),
            },
            page_layout_solver: Default::default(),
            preview: Default::default(),
            book_layout_solver: BookLayoutSolverConfig::default(),
        },
        photos: vec![PhotoGroup {
            group: "Test".to_owned(),
            sort_key: "2024-01-01".to_owned(),
            files: (0..10)
                .map(|i| PhotoFile {
                    id: format!("p{i}.jpg"),
                    source: format!("/photos/p{i}.jpg"),
                    timestamp: "2024-01-01T00:00:00Z".parse().unwrap(),
                    width_px: 4000,
                    height_px: 3000,
                    area_weight: 1.0,
                    hash: String::new(),
                })
                .collect(),
        }],
        layout,
    }
}

pub fn setup_repo(tmp: &TempDir, state: &ProjectState) {
    let repo = git::init_repo(tmp.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    drop(config);

    std::fs::write(tmp.path().join(".gitignore"), ".fotobuch/\n*.pdf\nlog*\n").unwrap();
    state.save(&tmp.path().join("urlaub.yaml")).unwrap();
    git::stage_and_commit(&repo, &[".gitignore", "urlaub.yaml"], "init").unwrap();
    git::create_branch(&repo, "fotobuch/urlaub").unwrap();
}
