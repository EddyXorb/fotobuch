use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use crate::models::{PhotoGroup, ScannedPhoto};

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tiff", "tif"];

/// Scans a root directory and returns all photo groups, sorted chronologically.
///
/// Each subdirectory becomes one group. The group timestamp is parsed from
/// the directory name, with EXIF data used as fallback per individual photo.
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

    // Sort groups chronologically; groups without a timestamp go last.
    groups.sort_by(|a, b| match (a.timestamp, b.timestamp) {
        (Some(ta), Some(tb)) => ta.cmp(&tb),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.label.cmp(&b.label),
    });

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
    let label = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let folder_timestamp = parse_timestamp_from_name(&label);
    debug!("Group {:?} -> timestamp: {:?}", label, folder_timestamp);

    let mut photos: Vec<ScannedPhoto> = read_photos(dir)?;

    // Enrich each photo with EXIF timestamp and dimensions.
    for photo in &mut photos {
        enrich_photo_metadata(photo);
        // Fall back to folder timestamp if EXIF is missing.
        if photo.timestamp.is_none() {
            photo.timestamp = folder_timestamp;
        }
    }

    // Sort photos within the group by timestamp.
    photos.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    Ok(PhotoGroup {
        label,
        timestamp: folder_timestamp,
        photos,
    })
}

/// Reads all supported image files from a directory (non-recursive).
fn read_photos(dir: &Path) -> Result<Vec<ScannedPhoto>> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("Cannot read {:?}", dir))?;

    let photos = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| is_supported_image(p))
        .map(ScannedPhoto::new)
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
fn enrich_photo_metadata(photo: &mut ScannedPhoto) {
    let file = match std::fs::File::open(&photo.path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Cannot open {:?}: {}", photo.path, e);
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
            && let Some(bytes) = vec.first() {
                let s = String::from_utf8_lossy(bytes);
                // EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
                if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
                    photo.timestamp = Some(dt);
                }
            }

    // Read pixel dimensions.
    let width = exif_u32(&exif, exif::Tag::PixelXDimension);
    let height = exif_u32(&exif, exif::Tag::PixelYDimension);
    if let (Some(w), Some(h)) = (width, height) {
        photo.dimensions = Some((w, h));
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
            && let Ok(dt) = NaiveDateTime::parse_from_str(&name[..*len], fmt) {
                return Some(dt);
            }
    }

    // Date-only formats: produce midnight timestamp.
    for (fmt, len) in &formats_date {
        if name.len() >= *len {
            let candidate = &name[..*len];
            if let Ok(date) = chrono::NaiveDate::parse_from_str(candidate, fmt) {
                return Some(date.and_hms_opt(0, 0, 0).unwrap());
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
