use serde::{Deserialize, Serialize};

use super::{BookConfig, BookLayoutSolverConfig, PageLayoutSolverConfig, PreviewConfig};

/// Complete project configuration as persisted in YAML
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    pub book: BookConfig,
    #[serde(default)]
    pub page_layout_solver: PageLayoutSolverConfig,
    #[serde(default)]
    pub preview: PreviewConfig,
    #[serde(default)]
    pub book_layout_solver: BookLayoutSolverConfig,
}
