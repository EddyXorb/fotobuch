mod photo_file;
mod photo_group;

pub use photo_file::PhotoFile;
pub use photo_group::PhotoGroup;

use std::collections::HashMap;

/// Maps photo ID → `(PhotoFile, group_name)` for fast lookup.
pub fn build_photo_index(photos: &[PhotoGroup]) -> HashMap<String, (PhotoFile, String)> {
    photos
        .iter()
        .flat_map(|group| {
            group
                .files
                .iter()
                .map(move |file| (file.id.clone(), (file.clone(), group.group.clone())))
        })
        .collect()
}
