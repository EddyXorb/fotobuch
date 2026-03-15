use regex::Regex;
use std::path::PathBuf;

use crate::dto_models::PhotoGroup;

/// Input parameters for scanning photos.
#[derive(Debug)]
pub struct ScannerInput {
    /// Paths to scan (files or directories)
    pub paths: Vec<PathBuf>,
    /// Optional filter for XMP metadata
    pub xmp_filter: Option<Regex>,
    /// Optional filter for source file path
    pub source_filter: Option<Regex>,
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
    pub xmp_filter: Option<Regex>,
    pub source_filter: Option<Regex>,
}
