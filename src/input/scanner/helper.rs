use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime};
use std::path::{Path, PathBuf};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tiff", "tif"];

pub fn naive_to_utc(naive: NaiveDateTime) -> DateTime<chrono::Utc> {
    DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc)
}

/// Returns all direct subdirectories of the given root path.
pub fn get_subdirs(root: &Path) -> Result<Vec<PathBuf>> {
    let entries =
        std::fs::read_dir(root).with_context(|| format!("Cannot read directory {:?}", root))?;

    let dirs = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    Ok(dirs)
}

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
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
