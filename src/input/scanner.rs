use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::dto_models::{PhotoFile, PhotoGroup};

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tiff", "tif"];

/// Scans a root directory and returns all photo groups, sorted chronologically.
///
/// Each subdirectory becomes one group. If the root directory itself contains photos,
/// they are grouped under the root directory's name.
pub fn scan_photo_dirs(root: &Path) -> Result<Vec<PhotoGroup>> {
    let mut groups: Vec<PhotoGroup> = read_subdirs(root)?
        .into_iter()
        .filter_map(|dir| match load_group(&dir) {
            Ok(group) => Some(group),
            Err(e) => {
                warn!("Skipping {:?}: {}", dir, e);
                None
            }
        })
        .collect();

    // Check if the root directory itself contains photos
    if let Ok(root_group) = load_group(root)
        && !root_group.files.is_empty()
    {
        groups.push(root_group);
    }

    // Sort groups according to sort_key (ISO 8601 timestamp string)
    groups.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    Ok(groups)
}

/// Returns all direct subdirectories of the given root path.
fn read_subdirs(root: &Path) -> Result<Vec<PathBuf>> {
    let entries =
        std::fs::read_dir(root).with_context(|| format!("Cannot read directory {:?}", root))?;

    let dirs = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    Ok(dirs)
}

/// Loads all photos from a directory and attempts to parse the folder timestamp.
fn load_group(dir: &Path) -> Result<PhotoGroup> {
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

    let mut photo_files: Vec<PhotoFile> = read_photos(dir, &group_name)?;

    // Enrich each photo with EXIF timestamp and dimensions.
    for photo in &mut photo_files {
        enrich_photo_metadata(photo);

        // Fall back to folder timestamp if EXIF is missing.
        if folder_timestamp.is_some() {
            let folder_dt = folder_timestamp
                .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
                .unwrap();

            // Use folder timestamp if photo timestamp is still placeholder (Utc::now())
            // We consider a timestamp "placeholder" if it's very recent (within 1 second of now)
            let now = Utc::now();
            if (now - photo.timestamp).num_seconds().abs() < 1 {
                photo.timestamp = folder_dt;
            }
        }
    }

    // Sort photos within the group by timestamp.
    photo_files.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // Determine sort_key from folder timestamp or earliest photo timestamp
    let sort_key = folder_timestamp
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc).to_rfc3339())
        .or_else(|| photo_files.first().map(|p| p.timestamp.to_rfc3339()))
        .unwrap_or_else(|| "9999-12-31T23:59:59Z".to_string());

    Ok(PhotoGroup {
        group: group_name,
        sort_key,
        files: photo_files,
    })
}

/// Reads all supported image files from a directory (non-recursive).
fn read_photos(dir: &Path, group_name: &str) -> Result<Vec<PhotoFile>> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("Cannot read {:?}", dir))?;

    let photos = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_supported_image(p))
        .enumerate()
        .map(|(idx, path)| {
            // Generate unique ID: "{group}/{filename_with_extension}"
            // The ID doubles as the relative cache path (per YAML schema).
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&format!("photo_{idx}.jpg"))
                .to_string();
            let id = format!("{group_name}/{filename}");

            PhotoFile {
                id,
                source: path.to_str().unwrap_or("").to_string(),
                width_px: 1,  // Placeholder, will be updated by enrich_photo_metadata
                height_px: 1, // Placeholder
                area_weight: 1.0,
                timestamp: Utc::now(), // Placeholder, will be updated
                hash: String::new(), // Will be computed by add command
            }
        })
        .collect();

    Ok(photos)
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Tries to read EXIF metadata from a photo to get timestamp and dimensions.
fn enrich_photo_metadata(photo: &mut PhotoFile) {
    let photo_path = PathBuf::from(&photo.source);

    // Try to read dimensions from image header first (fast, works for all formats)
    if let Ok(dimensions) = image::image_dimensions(&photo_path) {
        photo.width_px = dimensions.0;
        photo.height_px = dimensions.1;
    }

    let file = match std::fs::File::open(&photo_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Cannot open {:?}: {}", photo_path, e);
            return;
        }
    };

    let mut bufreader = std::io::BufReader::new(file);
    let exif_reader = exif::Reader::new();

    let exif = match exif_reader.read_from_container(&mut bufreader) {
        Ok(e) => e,
        Err(_) => return, // No EXIF — not an error, many PNGs lack it.
    };

    // Parse DateTimeOriginal from EXIF.
    if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        && let exif::Value::Ascii(ref vec) = field.value
        && let Some(bytes) = vec.first()
    {
        let s = String::from_utf8_lossy(bytes);
        // EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
        if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
            photo.timestamp = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
        }
    }

    // Read pixel dimensions from EXIF if not already read from header.
    if photo.width_px == 1 && photo.height_px == 1 {
        let width = exif_u32(&exif, exif::Tag::PixelXDimension);
        let height = exif_u32(&exif, exif::Tag::PixelYDimension);
        if let (Some(w), Some(h)) = (width, height) {
            photo.width_px = w;
            photo.height_px = h;
        }
    }
}

fn exif_u32(exif: &exif::Exif, tag: exif::Tag) -> Option<u32> {
    exif.get_field(tag, exif::In::PRIMARY)
        .and_then(|f| match f.value {
            exif::Value::Long(ref v) => v.first().copied(),
            exif::Value::Short(ref v) => v.first().map(|&x| x as u32),
            _ => None,
        })
}

/// Tries to parse a timestamp from a folder name.
///
/// Supported formats (examples):
/// - `2024-07-15`
/// - `2024-07-15_Urlaub`
/// - `20240715`
/// - `2024-07-15 18-30`
/// - `2024-07-15_18-30-00`
pub fn parse_timestamp_from_name(name: &str) -> Option<NaiveDateTime> {
    // Extract the leading date-like part (up to the first non-date character after the date).
    let formats_datetime = [
        ("%Y-%m-%d_%H-%M-%S", 19),
        ("%Y-%m-%d %H-%M", 16),
        ("%Y-%m-%d_%H-%M", 16),
        ("%Y%m%d_%H%M%S", 15),
        ("%Y%m%d_%H%M", 13),
        ("%Y-%m-%d@%H%M%S", 16),
    ];

    let formats_date = [("%Y-%m-%d", 10), ("%Y%m%d", 8), ("%Y_%m_%d", 10)];

    // Try to match from the start of the string.
    for (fmt, len) in &formats_datetime {
        if name.len() >= *len
            && let Ok(dt) = NaiveDateTime::parse_from_str(&name[..*len], fmt)
        {
            return Some(dt);
        }
    }

    // Date-only formats: produce midnight timestamp.
    for (fmt, len) in &formats_date {
        if name.len() >= *len {
            let candidate = &name[..*len];
            if let Ok(date) = chrono::NaiveDate::parse_from_str(candidate, fmt) {
                // and_hms_opt should always succeed for midnight (0:0:0)
                return date.and_hms_opt(0, 0, 0);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp_basic() {
        let ts = parse_timestamp_from_name("2024-07-15_Urlaub_Italien");
        assert!(ts.is_some());
        assert_eq!(ts.unwrap().date().to_string(), "2024-07-15");
    }

    #[test]
    fn test_parse_timestamp_compact() {
        let ts = parse_timestamp_from_name("20240715_Ferien");
        assert!(ts.is_some());
        assert_eq!(ts.unwrap().date().to_string(), "2024-07-15");
    }

    #[test]
    fn test_parse_timestamp_none() {
        let ts = parse_timestamp_from_name("Sonstiges");
        assert!(ts.is_none());
    }
}
