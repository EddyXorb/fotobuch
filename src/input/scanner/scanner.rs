use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::path::Path;
use tracing::{debug, warn};

use crate::dto_models::{PhotoFile, PhotoGroup};
use crate::input::xmp;

use super::helper::{get_subdirs, is_supported_image, naive_to_utc, parse_timestamp_from_name};
use super::metadata::enrich_photo_metadata;
use super::types::{ScanStats, ScannerFilters, ScannerInput};

/// Internal scanner with filtering logic.
pub struct Scanner {
    filters: ScannerFilters,
    pub stats: ScanStats,
}

impl Scanner {
    pub fn new(input: &ScannerInput) -> Self {
        Self {
            filters: ScannerFilters {
                xmp_filters: input.xmp_filters.clone(),
                source_filters: input.source_filters.clone(),
            },
            stats: ScanStats::default(),
        }
    }

    /// Scans a single image file and returns it as a photo group.
    ///
    /// The group name is derived from the file's parent directory name.
    /// If the file is unsupported or cannot be read, returns an empty vector.
    pub fn scan_single_file_photo_group(&mut self, path: &Path) -> Result<Vec<PhotoGroup>> {
        if !is_supported_image(path) {
            return Ok(Vec::new());
        }

        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("no_group")
            .to_string();

        let photo_opt = self.scan_single_photo(path, &parent, None);
        match photo_opt {
            Some(photo) => {
                let sort_key = photo.timestamp.to_rfc3339();
                Ok(vec![PhotoGroup {
                    group: parent,
                    sort_key,
                    files: vec![photo],
                }])
            }
            None => Ok(Vec::new()),
        }
    }

    /// Scans a root directory and returns all photo groups, sorted chronologically.
    ///
    /// Each subdirectory becomes one group. If the root directory itself contains photos,
    /// they are grouped under the root directory's name.
    pub fn scan_photo_group_dirs(&mut self, root: &Path) -> Result<Vec<PhotoGroup>> {
        let mut groups: Vec<PhotoGroup> = get_subdirs(root)?
            .into_iter()
            .filter_map(|dir| match self.scan_single_photo_group_dir(&dir) {
                Ok(group) => {
                    if !group.files.is_empty() {
                        Some(group)
                    } else {
                        None
                    }
                }
                Err(e) => {
                    warn!("Skipping {:?}: {}", dir, e);
                    None
                }
            })
            .collect();

        // Check if the root directory itself contains photos
        if let Ok(root_group) = self.scan_single_photo_group_dir(root)
            && !root_group.files.is_empty()
        {
            groups.push(root_group);
        }

        // Sort groups according to sort_key (ISO 8601 timestamp string)
        groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

        Ok(groups)
    }

    /// Loads all photos from a directory and attempts to parse the folder timestamp.
    fn scan_single_photo_group_dir(&mut self, dir: &Path) -> Result<PhotoGroup> {
        let group_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let folder_timestamp = parse_timestamp_from_name(&group_name);
        debug!(
            "Group {:?} -> timestamp: {:?}",
            group_name, folder_timestamp
        );

        let folder_dt = folder_timestamp.map(naive_to_utc);

        let mut photo_files = self.scan_photos_from_dir(dir, &group_name, folder_dt)?;

        // Sort photos within the group by timestamp.
        photo_files.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Determine sort_key from folder timestamp or earliest photo timestamp
        let sort_key = folder_dt
            .map(|dt| dt.to_rfc3339())
            .or_else(|| photo_files.first().map(|p| p.timestamp.to_rfc3339()))
            .unwrap_or_else(|| "9999-12-31T23:59:59Z".to_string());

        Ok(PhotoGroup {
            group: group_name,
            sort_key,
            files: photo_files,
        })
    }

    /// Reads all supported image files from a directory (non-recursive).
    fn scan_photos_from_dir(
        &mut self,
        dir: &Path,
        group_name: &str,
        folder_dt: Option<DateTime<Utc>>,
    ) -> Result<Vec<PhotoFile>> {
        let entries = std::fs::read_dir(dir).with_context(|| format!("Cannot read {:?}", dir))?;

        let photos = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| is_supported_image(p))
            .filter_map(|path| self.scan_single_photo(&path, group_name, folder_dt))
            .collect();

        Ok(photos)
    }

    /// Builds a PhotoFile from a path, enriches it with metadata, applies fallback timestamp,
    /// and applies filters. Returns None if photo is filtered out.
    /// All filters must match (AND logic).
    fn scan_single_photo(
        &mut self,
        path: &Path,
        group_name: &str,
        fallback_dt: Option<DateTime<Utc>>,
    ) -> Option<PhotoFile> {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.jpg")
            .to_string();

        let full_path = path.to_str().unwrap_or("").to_string();

        // Apply source filters (all must match)
        if !self.filters.source_filters.is_empty()
            && !self.filters.source_filters.iter().all(|pattern| pattern.is_match(&full_path)) {
                self.stats.source_filtered += 1;
                return None;
            }

        // Apply XMP filters (all must match)
        if !self.filters.xmp_filters.is_empty() {
            if !xmp::xmp_matches_all(Path::new(&full_path), &self.filters.xmp_filters)
                .unwrap_or(true)
            {
                self.stats.xmp_filtered += 1;
                return None;
            }
        }

        let mut photo = PhotoFile {
            id: format!("{group_name}/{filename}"),
            source: full_path,
            width_px: 1,
            height_px: 1,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: String::new(),
        };

        let found_timestamp = enrich_photo_metadata(&mut photo);
        if !found_timestamp && let Some(dt) = fallback_dt {
            photo.timestamp = dt;
        }

        Some(photo)
    }
}
