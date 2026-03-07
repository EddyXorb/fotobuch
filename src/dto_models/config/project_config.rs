use serde::{Deserialize, Serialize};

use super::{BookConfig, GaConfig, PreviewConfig};

/// Complete project configuration as persisted in YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub book: BookConfig,
    #[serde(default)]
    pub ga: GaConfig,
    #[serde(default)]
    pub preview: PreviewConfig,
}
