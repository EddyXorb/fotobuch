//! XMP metadata reading and filtering.
//!
//! Uses `xmpkit` (pure Rust) to extract the serialized XMP packet from image
//! files and match it against a user-supplied regex pattern.

use regex::Regex;
use std::path::Path;
use tracing::debug;
use xmpkit::XmpFile;

/// Returns `true` when the file's XMP packet contains a match for `pattern`.
///
/// Files without XMP metadata return None
pub fn xmp_matches(path: &Path, pattern: &Regex) -> Option<bool> {
    let Some(packet) = read_xmp_packet(path) else {
        debug!("No XMP found in {:?}, ", path);
        return None;
    };
    Some(pattern.is_match(&packet))
}

/// Reads and serializes the full XMP packet from a file as an XML string.
///
/// Returns `None` when the file contains no XMP metadata or cannot be read.
fn read_xmp_packet(path: &Path) -> Option<String> {
    let mut xmp_file = XmpFile::new();
    xmp_file.open_with(path, Default::default()).ok()?;
    xmp_file.get_xmp()?.serialize_packet().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_xmp_matches_no_xmp_returns_false() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"not an image").unwrap();
        let re = Regex::new(".*").unwrap();
        assert!(xmp_matches(f.path(), &re).is_none());
    }
}
