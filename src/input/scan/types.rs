use regex::Regex;
use std::path::PathBuf;

use crate::dto_models::PhotoGroup;

/// Input parameters for scanning photos.
#[derive(Debug)]
pub struct ScannerInput {
    /// Paths to scan (files or directories)
    pub paths: Vec<PathBuf>,
    /// Filters for XMP metadata (all must match)
    pub xmp_filters: Vec<Regex>,
    /// Filters for source file path (all must match)
    pub source_filters: Vec<Regex>,
    /// Scan directories recursively (each subdir becomes its own group)
    pub recursive: bool,
}

/// Statistics from photo scanning.
#[derive(Debug, Default)]
pub struct ScanStats {
    /// Number of photos filtered by XMP metadata
    pub xmp_filtered: usize,
    /// Number of photos filtered by source path
    pub source_filtered: usize,
}

/// Output from scanning photos.
#[derive(Debug)]
pub struct ScannerOutput {
    /// Photo groups discovered during scan
    pub groups: Vec<PhotoGroup>,
    /// Filtering statistics
    pub stats: ScanStats,
}

/// Filter configuration extracted from ScannerInput.
#[derive(Debug)]
pub struct ScannerFilters {
    pub xmp_filters: Vec<Regex>,
    pub source_filters: Vec<Regex>,
}
