//! Duplicate detection for photo files

use crate::dto_models::PhotoFile;
use crate::input::metadata::compute_partial_hash;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Filters duplicates out of a file list.
///
/// Returns `(kept, updated, skipped, warnings)`.
///
/// - `kept`: new files to add
/// - `updated`: files whose path matched an existing entry but whose hash changed
/// - `skipped`: count of files that are unchanged duplicates
/// - `warnings`: hash-based duplicate warnings or hash failures
///
/// When `update=true`, path-matched files are hash-checked against
/// `existing_path_hashes`; if the hash differs the new file is returned in
/// `updated` instead of being skipped.
pub fn deduplicate(
    files: &mut Vec<PhotoFile>,
    existing_paths: &HashSet<PathBuf>,
    existing_hashes: &HashSet<String>,
    allow_duplicates: bool,
    update: bool,
    existing_path_hashes: &HashMap<PathBuf, String>,
) -> (Vec<PhotoFile>, Vec<PhotoFile>, usize, Vec<String>) {
    let mut kept = Vec::new();
    let mut updated = Vec::new();
    let mut skipped = 0;
    let mut warnings = Vec::new();

    for mut file in files.drain(..) {
        let path = PathBuf::from(&file.source);

        if existing_paths.contains(&path) {
            if update {
                // Re-hash and compare to decide if content changed
                match compute_partial_hash(&path) {
                    Ok(hash) => {
                        let old_hash = existing_path_hashes.get(&path).map(String::as_str).unwrap_or("");
                        if hash == old_hash {
                            skipped += 1;
                        } else {
                            file.hash = hash;
                            updated.push(file);
                        }
                    }
                    Err(e) => {
                        warnings.push(format!("Hash failed for {}: {}", path.display(), e));
                    }
                }
            } else {
                skipped += 1;
            }
            continue;
        }

        // Compute hash and check for content duplicates among new files
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

    (kept, updated, skipped, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::PhotoFile;
    use crate::input::metadata::compute_partial_hash;
    use chrono::Utc;
    use std::collections::{HashMap, HashSet};
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

    fn no_update(
        files: &mut Vec<PhotoFile>,
        existing_paths: &HashSet<PathBuf>,
        existing_hashes: &HashSet<String>,
        allow_duplicates: bool,
    ) -> (Vec<PhotoFile>, Vec<PhotoFile>, usize, Vec<String>) {
        deduplicate(files, existing_paths, existing_hashes, allow_duplicates, false, &HashMap::new())
    }

    #[test]
    fn test_deduplicate_empty_list() {
        let mut files = vec![];
        let existing_paths = HashSet::new();
        let existing_hashes = HashSet::new();

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(updated.len(), 0);
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

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(updated.len(), 0);
        assert_eq!(skipped, 1);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_deduplicate_hash_duplicate_disallowed() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let hash = compute_partial_hash(temp_file.path()).unwrap();

        let mut files = vec![create_test_photo("photo1", path, "")];

        let existing_paths = HashSet::new();
        let mut existing_hashes = HashSet::new();
        existing_hashes.insert(hash);

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(updated.len(), 0);
        assert_eq!(skipped, 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Duplicate (by hash)"));
    }

    #[test]
    fn test_deduplicate_hash_duplicate_allowed() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let hash = compute_partial_hash(temp_file.path()).unwrap();

        let mut files = vec![create_test_photo("photo1", path, "")];

        let existing_paths = HashSet::new();
        let mut existing_hashes = HashSet::new();
        existing_hashes.insert(hash);

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, true);

        assert_eq!(kept.len(), 1);
        assert_eq!(updated.len(), 0);
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

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 1);
        assert_eq!(updated.len(), 0);
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

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 0);
        assert_eq!(updated.len(), 0);
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

        let mut existing_paths = HashSet::new();
        existing_paths.insert(PathBuf::from(path1));
        let existing_hashes = HashSet::new();

        let (kept, updated, skipped, warnings) =
            no_update(&mut files, &existing_paths, &existing_hashes, false);

        assert_eq!(kept.len(), 1); // only photo2
        assert_eq!(updated.len(), 0);
        assert_eq!(skipped, 1); // photo1 skipped by path
        assert_eq!(warnings.len(), 0);
        assert_eq!(kept[0].id, "photo2");
    }

    #[test]
    fn test_deduplicate_update_unchanged_file() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"original content").unwrap();
        let path = PathBuf::from(temp_file.path());
        let hash = compute_partial_hash(&path).unwrap();

        let mut files = vec![create_test_photo("photo1", path.to_str().unwrap(), "")];

        let mut existing_paths = HashSet::new();
        existing_paths.insert(path.clone());
        let existing_hashes = HashSet::new();
        let mut existing_path_hashes = HashMap::new();
        existing_path_hashes.insert(path, hash);

        let (kept, updated, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false, true, &existing_path_hashes);

        assert_eq!(kept.len(), 0);
        assert_eq!(updated.len(), 0); // same hash → still skipped
        assert_eq!(skipped, 1);
        assert_eq!(warnings.len(), 0);
    }

    #[test]
    fn test_deduplicate_update_changed_file() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"new content after lightroom export").unwrap();
        let path = PathBuf::from(temp_file.path());
        let new_hash = compute_partial_hash(&path).unwrap();

        let mut files = vec![create_test_photo("photo1", path.to_str().unwrap(), "")];

        let mut existing_paths = HashSet::new();
        existing_paths.insert(path.clone());
        let existing_hashes = HashSet::new();
        // Store the OLD (different) hash
        let mut existing_path_hashes = HashMap::new();
        existing_path_hashes.insert(path, "old_hash_that_does_not_match".to_string());

        let (kept, updated, skipped, warnings) =
            deduplicate(&mut files, &existing_paths, &existing_hashes, false, true, &existing_path_hashes);

        assert_eq!(kept.len(), 0);
        assert_eq!(updated.len(), 1); // hash changed → updated
        assert_eq!(skipped, 0);
        assert_eq!(warnings.len(), 0);
        assert_eq!(updated[0].hash, new_hash);
    }
}
