//! Duplicate detection for photo files

use crate::dto_models::PhotoFile;
use crate::input::metadata::compute_partial_hash;
use std::collections::HashSet;
use std::path::PathBuf;

/// Filters duplicates out of a file list.
///
/// Returns (kept files, skip count, warnings).
///
/// # Arguments
/// * `files` - Mutable list of photo files to filter (will be drained)
/// * `existing_paths` - Set of paths that already exist in the project
/// * `existing_hashes` - Set of hashes that already exist in the project
/// * `allow_duplicates` - If true, hash duplicates are allowed
pub fn deduplicate(
    files: &mut Vec<PhotoFile>,
    existing_paths: &HashSet<PathBuf>,
    existing_hashes: &HashSet<String>,
    allow_duplicates: bool,
) -> (Vec<PhotoFile>, usize, Vec<String>) {
    let mut kept = Vec::new();
    let mut skipped = 0;
    let mut warnings = Vec::new();

    for mut file in files.drain(..) {
        let path = PathBuf::from(&file.source);

        // Path check
        if existing_paths.contains(&path) {
            skipped += 1;
            continue;
        }

        // Compute hash and check
        match compute_partial_hash(&path) {
            Ok(hash) => {
                if !allow_duplicates && existing_hashes.contains(&hash) {
                    warnings.push(format!("Duplicate (by hash): {}", path.display()));
                    skipped += 1;
                    continue;
                }
                file.hash = hash;
            }
            Err(e) => {
                warnings.push(format!("Hash failed for {}: {}", path.display(), e));
                continue;
            }
        }

        kept.push(file);
    }

    (kept, skipped, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::PhotoFile;
    use crate::input::metadata::compute_partial_hash;
    use chrono::Utc;
    use std::collections::HashSet;
    use tempfile::NamedTempFile;

    fn create_test_photo(id: &str, source: &str, hash: &str) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: source.to_string(),
            hash: hash.to_string(),
            width_px: 1920,
            height_px: 1080,
            area_weight: 1.0,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_deduplicate_empty_list() {
        let mut files = vec![];
        let existing_paths = HashSet::new();
        let existing_hashes = HashSet::new();

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(skipped, 0);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_deduplicate_path_duplicate() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut files = vec![create_test_photo("photo1", path, "")];

        let mut existing_paths = HashSet::new();
        existing_paths.insert(PathBuf::from(path));
        let existing_hashes = HashSet::new();

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(skipped, 1);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_deduplicate_hash_duplicate_disallowed() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Compute the actual hash
        let hash = compute_partial_hash(temp_file.path()).unwrap();

        let mut files = vec![create_test_photo("photo1", path, "")];

        let existing_paths = HashSet::new();
        let mut existing_hashes = HashSet::new();
        existing_hashes.insert(hash);

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(skipped, 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Duplicate (by hash)"));
    }

    #[test]
    fn test_deduplicate_hash_duplicate_allowed() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Compute the actual hash
        let hash = compute_partial_hash(temp_file.path()).unwrap();

        let mut files = vec![create_test_photo("photo1", path, "")];

        let existing_paths = HashSet::new();
        let mut existing_hashes = HashSet::new();
        existing_hashes.insert(hash);

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, true);

        assert_eq!(kept.len(), 1);
        assert_eq!(skipped, 0);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_deduplicate_hash_computed_and_stored() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut files = vec![create_test_photo("photo1", path, "")];

        let existing_paths = HashSet::new();
        let existing_hashes = HashSet::new();

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 1);
        assert_eq!(skipped, 0);
        assert_eq!(warnings.len(), 0);
        assert!(!kept[0].hash.is_empty());
        assert_eq!(kept[0].hash.len(), 64);
    }

    #[test]
    fn test_deduplicate_missing_file() {
        let mut files = vec![create_test_photo("photo1", "/nonexistent/file.jpg", "")];

        let existing_paths = HashSet::new();
        let existing_hashes = HashSet::new();

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(skipped, 0);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Hash failed"));
    }

    #[test]
    fn test_deduplicate_mixed_scenario() {
        let temp_file1 = NamedTempFile::new().unwrap();
        let temp_file2 = NamedTempFile::new().unwrap();
        std::fs::write(temp_file1.path(), b"content1").unwrap();
        std::fs::write(temp_file2.path(), b"content2").unwrap();

        let path1 = temp_file1.path().to_str().unwrap();
        let path2 = temp_file2.path().to_str().unwrap();

        let mut files = vec![
            create_test_photo("photo1", path1, ""),
            create_test_photo("photo2", path2, ""),
        ];

        // path1 already exists by path
        let mut existing_paths = HashSet::new();
        existing_paths.insert(PathBuf::from(path1));
        let existing_hashes = HashSet::new();

        let (kept, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 1); // only photo2
        assert_eq!(skipped, 1); // photo1 skipped by path
        assert_eq!(warnings.len(), 0);
        assert_eq!(kept[0].id, "photo2");
    }
}
