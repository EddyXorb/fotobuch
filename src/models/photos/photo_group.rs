use serde::{Deserialize, Serialize};

use super::PhotoFile;

/// Group of photos from a single directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoGroup {
    /// Group name (relative path from add argument)
    pub group: String,
    /// Timestamp for chronological ordering (ISO 8601)
    pub sort_key: String,
    /// Photos in this group
    pub files: Vec<PhotoFile>,
}
