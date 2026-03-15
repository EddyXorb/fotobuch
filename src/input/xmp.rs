//! XMP metadata reading and filtering.
//!
//! Custom parser to extract the serialized XMP packet from image files
//! and match it against a user-supplied regex pattern.

use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::debug;

/// Returns `true` when the file's XMP packet contains a match for `pattern`.
///
/// Files without XMP metadata return None
pub fn xmp_matches(path: &Path, pattern: &Regex) -> Option<bool> {
    let Some(packet) = read_xmp_packet(path) else {
        debug!("No XMP found in {:?}", path);
        return None;
    };

    Some(pattern.is_match(&packet))
}

/// Reads the full XMP packet from a file as an XML string.
///
/// Extracts the XMP content between `<?xpacket` and `?>` markers.
/// Returns `None` when the file contains no XMP metadata or cannot be read.
fn read_xmp_packet(path: &Path) -> Option<String> {
    const CHUNK_SIZE: usize = 64 * 1024;
    const OVERLAP: usize = 14; // max marker len - 1

    let begin_marker = b"<?xpacket begin";
    let end_marker = b"<?xpacket end";
    let pi_close = b"?>";

    let file = File::open(path).ok()?;
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, file);

    // Phase 1: Find begin marker, discard everything before it
    let mut window = Vec::new();
    loop {
        let available = reader.fill_buf().ok()?;
        if available.is_empty() {
            return None;
        }
        let len = available.len();
        window.extend_from_slice(available);
        reader.consume(len);

        if let Some(pos) = window
            .windows(begin_marker.len())
            .position(|w| w == begin_marker)
        {
            window.drain(..pos);
            break;
        }

        // Keep only overlap to catch markers spanning chunk boundaries
        let drain_to = window.len().saturating_sub(OVERLAP);
        window.drain(..drain_to);
    }

    // Phase 2: Accumulate from begin marker until end marker + pi_close found
    loop {
        if let Some(end_start) = window[begin_marker.len()..]
            .windows(end_marker.len())
            .position(|w| w == end_marker)
        {
            let end_tag_start = begin_marker.len() + end_start;
            if let Some(close_pos) = window[end_tag_start..]
                .windows(pi_close.len())
                .position(|w| w == pi_close)
            {
                let end = end_tag_start + close_pos + pi_close.len();
                return std::str::from_utf8(&window[..end]).ok().map(str::to_string);
            }
        }

        let available = reader.fill_buf().ok()?;
        if available.is_empty() {
            return None;
        }
        let len = available.len();
        window.extend_from_slice(available);
        reader.consume(len);
    }
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

    #[test]
    fn test_read_xmp_packet_extracts_full_packet() {
        let path = std::path::Path::new("tests/fixtures/test_photo_xmp/quark_with_xmp.jpg");
        if path.exists() {
            let result = read_xmp_packet(path).unwrap();
            assert!(result.starts_with("<?xpacket begin"));
            assert!(result.ends_with("?>"));
            assert!(result.contains("<x:xmpmeta"));
            assert!(result.contains("Test XMP description for fotobuch"));
        }
    }

    #[test]
    fn test_read_xmp_packet_no_xmp_returns_none() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"just some random bytes").unwrap();
        assert!(read_xmp_packet(f.path()).is_none());
    }
}
