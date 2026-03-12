//! Photo metadata hashing for duplicate detection.
//!
//! This module extends the existing scanner module with partial hashing
//! functionality for efficient duplicate detection.

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Compute partial hash: first 64KB + last 64KB + file size
///
/// This is much faster than hashing the entire file and sufficient for
/// detecting duplicate photos while minimizing false positives.
pub fn compute_partial_hash(path: &Path) -> Result<String> {
    const CHUNK_SIZE: usize = 64 * 1024; // 64KB

    let mut file = File::open(path)
        .with_context(|| format!("Failed to open file for hashing: {}", path.display()))?;

    let file_size = file.metadata()?.len();

    let mut hasher = blake3::Hasher::new();

    // Hash first chunk
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let first_bytes_read = file.read(&mut buffer)?;
    hasher.update(&buffer[..first_bytes_read]);

    // Hash last chunk if file > CHUNK_SIZE
    if file_size > CHUNK_SIZE as u64 {
        file.seek(SeekFrom::End(-(CHUNK_SIZE as i64)))?;
        let last_bytes_read = file.read(&mut buffer)?;
        hasher.update(&buffer[..last_bytes_read]);
    }

    // Include file size in hash to differentiate files with same content but different sizes
    hasher.update(&file_size.to_le_bytes());

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_partial_hash_consistency() {
        // Create a test file
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile
            .write_all(b"Hello, world! This is test data.")
            .unwrap();
        tmpfile.flush().unwrap();

        let hash1 = compute_partial_hash(tmpfile.path()).unwrap();
        let hash2 = compute_partial_hash(tmpfile.path()).unwrap();

        assert_eq!(hash1, hash2, "Hash should be consistent");
    }

    #[test]
    fn test_partial_hash_different_files() {
        let mut file1 = NamedTempFile::new().unwrap();
        file1.write_all(b"Content A").unwrap();
        file1.flush().unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        file2.write_all(b"Content B").unwrap();
        file2.flush().unwrap();

        let hash1 = compute_partial_hash(file1.path()).unwrap();
        let hash2 = compute_partial_hash(file2.path()).unwrap();

        assert_ne!(hash1, hash2, "Different files should have different hashes");
    }

    #[test]
    fn test_partial_hash_large_file() {
        // Create a file larger than 128KB
        let mut tmpfile = NamedTempFile::new().unwrap();
        let large_data = vec![0xAB; 200 * 1024]; // 200KB
        tmpfile.write_all(&large_data).unwrap();
        tmpfile.flush().unwrap();

        let hash = compute_partial_hash(tmpfile.path()).unwrap();
        assert!(!hash.is_empty());
    }
}
