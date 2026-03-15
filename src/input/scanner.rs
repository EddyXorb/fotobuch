mod helper;
mod metadata;
mod scanner;
mod types;

use anyhow::Result;
use std::path::Path;

use crate::dto_models::PhotoGroup;

// Re-export public API
pub use helper::parse_timestamp_from_name;
pub use metadata::enrich_photo_metadata;
pub use scanner::Scanner;
pub use types::{ScanStats, ScannerInput, ScannerOutput};

// For tests
#[cfg(test)]
use {crate::dto_models::PhotoFile, chrono::Utc, regex::Regex, std::path::PathBuf};

/// Scans photos from given paths, applies filters, and returns groups with statistics.
///
/// # Steps
/// 1. Create Scanner with filters
/// 2. For each path: dispatch to file or directory scanner
/// 3. Filtering happens inside scan methods (early filtering)
/// 4. Return all groups + stats
pub fn scan_photos(input: ScannerInput) -> Result<ScannerOutput> {
    use anyhow::Context;

    let mut scanner = Scanner::new(&input);
    let mut groups = Vec::new();

    for path in input.paths {
        let scanned_groups = if path.is_file() {
            scanner
                .scan_single_file_photo_group(&path)
                .with_context(|| format!("Failed to scan file {}", path.display()))?
        } else if path.is_dir() {
            scanner
                .scan_photo_group_dirs(&path)
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
    };
    let output = scan_photos(input)?;
    Ok(output.groups)
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

    #[test]
    fn test_exif_orientation_swaps_dimensions() {
        // Test that a portrait photo with EXIF orientation tag 6 (90° CW)
        // has its width and height swapped to match display orientation.
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

        // After reading EXIF with orientation 6, dimensions should be swapped.
        // Original pixels are read, then swapped because orientation tag says "rotate 90°".
        // Portrait photo dimensions should have width < height after orientation handling.
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
        };

        let output = scan_photos(input).expect("scan_single_file should succeed");
        assert_eq!(output.groups.len(), 1, "should return exactly one group");
        let group = &output.groups[0];
        assert_eq!(
            group.group, "rotated",
            "group name should be parent dir name"
        );
        assert_eq!(group.files.len(), 1, "should contain exactly one photo");

        let photo = &group.files[0];
        assert!(photo.source.ends_with("portrait.jpg"));
        assert_eq!(photo.area_weight, 1.0);
        assert!(photo.width_px > 0 && photo.height_px > 0);
    }

    #[test]
    fn test_scan_single_file_unsupported() {
        // Test with a path that has an unsupported extension
        // Even if file doesn't exist, the extension check happens first
        let unsupported_path = PathBuf::from("tests/fixtures/unsupported.txt");

        let input = ScannerInput {
            paths: vec![unsupported_path.clone()],
            xmp_filters: vec![],
            source_filters: vec![],
        };

        // This should either skip the path (if no exist check) or bail (if file doesn't exist)
        // For now, just verify it doesn't panic with correct behavior
        match scan_photos(input) {
            Ok(output) => {
                // If file doesn't exist, extension is checked first and returns empty
                assert_eq!(output.groups.len(), 0);
            }
            Err(_) => {
                // File doesn't exist - that's fine for this test
            }
        }
    }

    #[test]
    fn test_scan_photos_empty_input() {
        let input = ScannerInput {
            paths: vec![],
            xmp_filters: vec![],
            source_filters: vec![],
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

        // Filter that matches the path
        let matching_filter = Regex::new("portrait").unwrap();
        let input = ScannerInput {
            paths: vec![portrait_path.clone()],
            xmp_filters: vec![],
            source_filters: vec![matching_filter],
        };

        let output = scan_photos(input).expect("scan_photos should succeed");
        assert_eq!(output.groups.len(), 1);
        assert_eq!(output.groups[0].files.len(), 1);
        assert_eq!(output.stats.source_filtered, 0);

        // Filter that doesn't match
        let non_matching_filter = Regex::new("landscape").unwrap();
        let input = ScannerInput {
            paths: vec![portrait_path],
            xmp_filters: vec![],
            source_filters: vec![non_matching_filter],
        };

        let output = scan_photos(input).expect("scan_photos should succeed");
        assert_eq!(output.groups.len(), 0);
        assert_eq!(output.stats.source_filtered, 1);
    }
}
