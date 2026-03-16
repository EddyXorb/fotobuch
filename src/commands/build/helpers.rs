use tracing::info;

use crate::dto_models::{PhotoFile, PhotoGroup, ProjectState};
use crate::output::typst;
use std::{collections::HashMap, path::Path};

pub fn update_preview_pdf(
    project_root: &Path,
    bleed_mm: f64,
    project_name: &str,
) -> Result<std::path::PathBuf, anyhow::Error> {
    let pdf_path = typst::compile_preview(project_root, project_name, bleed_mm)?;
    info!("PDF updated: {}", pdf_path.display());
    Ok(pdf_path)
}

/// Maps photo ID to (PhotoFile, group_name).
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

/// Sammelt alle Fotos aus dem Seitenbereich und rekonstruiert PhotoGroups.
///
/// start: 0-basiert (inclusive)
/// end: 1-basiert (= exklusiv, passt zu layout[start..end] und splice)
pub fn collect_photos_as_groups(state: &ProjectState, start: usize, end: usize) -> Vec<PhotoGroup> {
    let photo_index = build_photo_index(&state.photos);

    // Photo-IDs aus dem Bereich sammeln
    let page_photo_ids: Vec<&str> = state.layout[start..end]
        .iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    // Nach Originalgruppe aufteilen
    let mut groups_map: HashMap<&str, Vec<PhotoFile>> = HashMap::new();
    for id in &page_photo_ids {
        if let Some((pf, group_name)) = photo_index.get(*id) {
            groups_map
                .entry(group_name)
                .or_default()
                .push((*pf).clone());
        }
    }

    // sort_key aus state.photos übernehmen
    let group_sort_keys: HashMap<&str, &str> = state
        .photos
        .iter()
        .map(|g| (g.group.as_str(), g.sort_key.as_str()))
        .collect();

    let mut groups: Vec<PhotoGroup> = groups_map
        .into_iter()
        .map(|(name, files)| PhotoGroup {
            group: name.to_string(),
            sort_key: group_sort_keys.get(name).unwrap_or(&"").to_string(),
            files,
        })
        .collect();

    groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    groups
}
