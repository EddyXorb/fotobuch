mod helper;
mod metadata;
mod scanner;
mod types;

use anyhow::{Context, Result};
use std::path::Path;

use crate::dto_models::PhotoGroup;

use scanner::Scanner;

// Re-export public API
pub use helper::parse_timestamp_from_name;
pub use metadata::enrich_photo_metadata;
pub use types::{ScanStats, ScannerFilters, ScannerInput, ScannerOutput};

#[cfg(test)]
use {chrono::Utc, regex::Regex, std::path::PathBuf};

/// Scans photos from given paths, applies filters, and returns groups with statistics.
pub fn scan_photos(input: ScannerInput) -> Result<ScannerOutput> {
    let mut scanner = Scanner::new(&input);
    let mut groups = Vec::new();

    for path in input.paths {
        let scanned_groups = if path.is_file() {
            scanner
                .scan_single_file_photo_group(&path)
                .with_context(|| format!("Failed to scan file {}", path.display()))?
        } else if path.is_dir() {
            scanner
                .scan_photo_group_dirs(&path, input.recursive)
                .with_context(|| format!("Failed to scan directory {}", path.display()))?
        } else {
            anyhow::bail!("Path is neither a file nor a directory: {}", path.display());
        };

        for group in scanned_groups {
            if !group.files.is_empty() {
                groups.push(group);
            }
        }
    }

    Ok(ScannerOutput {
        groups,
        stats: scanner.stats,
    })
}

/// Public wrapper for scanning a directory without filters.
/// Used by the loader module for backwards compatibility.
pub fn scan_photo_group_dirs(root: &Path) -> Result<Vec<PhotoGroup>> {
    let input = ScannerInput {
        paths: vec![root.to_path_buf()],
        xmp_filters: vec![],
        source_filters: vec![],
        recursive: false,
    };
    let output = scan_photos(input)?;
    Ok(output.groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::PhotoFile;

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

    #[test]
    fn test_exif_orientation_swaps_dimensions() {
        let portrait_path = PathBuf::from("tests/fixtures/rotated/portrait.jpg");

        if !portrait_path.exists() {
            eprintln!("Test fixture not found: {:?}", portrait_path);
            return;
        }

        let mut photo = PhotoFile {
            id: "test/portrait.jpg".to_string(),
            source: portrait_path.to_string_lossy().to_string(),
            width_px: 1,
            height_px: 1,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: String::new(),
        };

        enrich_photo_metadata(&mut photo);

        assert!(photo.width_px > 0, "width should be set");
        assert!(photo.height_px > 0, "height should be set");
        assert!(
            photo.width_px < photo.height_px,
            "Portrait photo should have width < height (got {}x{})",
            photo.width_px,
            photo.height_px
        );
    }

    #[test]
    fn test_scan_single_file_valid() {
        let portrait_path = PathBuf::from("tests/fixtures/rotated/portrait.jpg");

        if !portrait_path.exists() {
            eprintln!("Test fixture not found: {:?}", portrait_path);
            return;
        }

        let input = ScannerInput {
            paths: vec![portrait_path.clone()],
            xmp_filters: vec![],
            source_filters: vec![],
            recursive: false,
        };

        let output = scan_photos(input).expect("scan_single_file should succeed");
        assert_eq!(output.groups.len(), 1, "should return exactly one group");
        let group = &output.groups[0];
        assert_eq!(group.group, "rotated", "group name should be parent dir name");
        assert_eq!(group.files.len(), 1, "should contain exactly one photo");

        let photo = &output.groups[0].files[0];
        assert!(photo.source.ends_with("portrait.jpg"));
        assert_eq!(photo.area_weight, 1.0);
        assert!(photo.width_px > 0 && photo.height_px > 0);
    }

    #[test]
    fn test_scan_single_file_unsupported() {
        let unsupported_path = PathBuf::from("tests/fixtures/unsupported.txt");

        let input = ScannerInput {
            paths: vec![unsupported_path],
            xmp_filters: vec![],
            source_filters: vec![],
            recursive: false,
        };

        match scan_photos(input) {
            Ok(output) => assert_eq!(output.groups.len(), 0),
            Err(_) => {}
        }
    }

    #[test]
    fn test_scan_photos_empty_input() {
        let input = ScannerInput {
            paths: vec![],
            xmp_filters: vec![],
            source_filters: vec![],
            recursive: false,
        };

        let output = scan_photos(input).expect("scan_photos should handle empty paths");
        assert_eq!(output.groups.len(), 0);
        assert_eq!(output.stats.xmp_filtered, 0);
        assert_eq!(output.stats.source_filtered, 0);
    }

    #[test]
    fn test_scan_photos_source_filter() {
        let portrait_path = PathBuf::from("tests/fixtures/rotated/portrait.jpg");

        if !portrait_path.exists() {
            eprintln!("Test fixture not found: {:?}", portrait_path);
            return;
        }

        let matching_filter = Regex::new("portrait").unwrap();
        let input = ScannerInput {
            paths: vec![portrait_path.clone()],
            xmp_filters: vec![],
            source_filters: vec![matching_filter],
            recursive: false,
        };

        let output = scan_photos(input).expect("scan_photos should succeed");
        assert_eq!(output.groups.len(), 1);
        assert_eq!(output.groups[0].files.len(), 1);
        assert_eq!(output.stats.source_filtered, 0);

        let non_matching_filter = Regex::new("landscape").unwrap();
        let input = ScannerInput {
            paths: vec![portrait_path],
            xmp_filters: vec![],
            source_filters: vec![non_matching_filter],
            recursive: false,
        };

        let output = scan_photos(input).expect("scan_photos should succeed");
        assert_eq!(output.groups.len(), 0);
        assert_eq!(output.stats.source_filtered, 1);
    }
}
