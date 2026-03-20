use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime};
use std::path::{Path, PathBuf};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tiff", "tif"];

pub fn naive_to_utc(naive: NaiveDateTime) -> DateTime<chrono::Utc> {
    DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc)
}

/// Returns all directories reachable from `root` (including `root` itself), recursively.
pub fn get_all_dirs_recursive(root: &Path) -> Result<Vec<PathBuf>> {
    let mut result = vec![root.to_path_buf()];
    let mut queue = vec![root.to_path_buf()];

    while let Some(dir) = queue.pop() {
        let entries = std::fs::read_dir(&dir)
            .with_context(|| format!("Cannot read directory {:?}", dir))?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                result.push(path.clone());
                queue.push(path);
            }
        }
    }

    Ok(result)
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
        ("%Y-%m-%d_%H-%M-%S", "2024-07-15_18-30-00".len()),
        ("%Y-%m-%d %H-%M",    "2024-07-15 18-30".len()),
        ("%Y-%m-%d_%H-%M",    "2024-07-15_18-30".len()),
        ("%Y%m%d_%H%M%S",     "20240715_183000".len()),
        ("%Y%m%d_%H%M",       "20240715_1830".len()),
        ("%Y-%m-%d@%H%M%S",   "2024-07-15@183000".len()),
    ];

    let formats_date = [
        ("%Y-%m-%d", "2024-07-15".len()),
        ("%Y%m%d",   "20240715".len()),
        ("%Y_%m_%d", "2024_07_15".len()),
    ];

    // Try to match from the start of the string.
    for (fmt, len) in &formats_datetime {
        if let Some(candidate) = name.get(..*len)
            && let Ok(dt) = NaiveDateTime::parse_from_str(candidate, fmt)
        {
            return Some(dt);
        }
    }

    // Date-only formats: produce midnight timestamp.
    for (fmt, len) in &formats_date {
        if let Some(candidate) = name.get(..*len)
            && let Ok(date) = chrono::NaiveDate::parse_from_str(candidate, fmt) {
                return date.and_hms_opt(0, 0, 0);
            }
    }

    None
}
