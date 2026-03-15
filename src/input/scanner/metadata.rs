use chrono::NaiveDateTime;
use image::ImageReader;
use image::metadata::Orientation;
use std::path::PathBuf;
use tracing::warn;

use super::helper::{naive_to_utc, parse_timestamp_from_name};
use crate::dto_models::PhotoFile;

/// Tries to read EXIF metadata from a photo to get timestamp and dimensions.
/// Returns true if a real timestamp was found (EXIF or filename), false if only placeholder.
pub fn enrich_photo_metadata(photo: &mut PhotoFile) -> bool {
    use image::ImageDecoder;
    let photo_path = PathBuf::from(&photo.source);

    // Try to read dimensions from image header first (fast, works for all formats)
    if let Ok(dimensions) = image::image_dimensions(&photo_path) {
        photo.width_px = dimensions.0;
        photo.height_px = dimensions.1;
    }

    // Read EXIF orientation using ImageReader API
    if let Ok(reader) = ImageReader::open(&photo_path)
        && let Ok(mut decoder) = reader.into_decoder()
        && let Ok(orientation) = decoder.orientation()
    {
        // Swap dimensions if orientation requires 90° or 270° rotation
        match orientation {
            Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH => {
                std::mem::swap(&mut photo.width_px, &mut photo.height_px);
            }
            _ => {} // Other orientations don't require dimension swapping
        }
    }

    // Read EXIF timestamp using exif crate
    let file = match std::fs::File::open(&photo_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Cannot open {:?}: {}", photo_path, e);
            return false;
        }
    };

    let mut bufreader = std::io::BufReader::new(file);
    let exif_reader = exif::Reader::new();

    let mut found_timestamp = false;

    if let Ok(exif) = exif_reader.read_from_container(&mut bufreader) {
        // Parse DateTimeOriginal from EXIF.
        if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
            && let exif::Value::Ascii(ref vec) = field.value
            && let Some(bytes) = vec.first()
        {
            let s = String::from_utf8_lossy(bytes);
            // EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
            if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y:%m:%d %H:%M:%S") {
                photo.timestamp = naive_to_utc(dt);
                found_timestamp = true;
            }
        }

        // Read pixel dimensions from EXIF if not already read from header.
        if photo.width_px == 1 && photo.height_px == 1 {
            let width = exif_u32(&exif, exif::Tag::PixelXDimension);
            let height = exif_u32(&exif, exif::Tag::PixelYDimension);
            if let (Some(w), Some(h)) = (width, height) {
                photo.width_px = w;
                photo.height_px = h;
            }
        }
    }

    // Fallback: parse timestamp from filename if EXIF had none.
    if !found_timestamp
        && let Some(stem) = photo_path.file_stem().and_then(|s| s.to_str())
        && let Some(dt) = parse_timestamp_from_name(stem)
    {
        photo.timestamp = naive_to_utc(dt);
        found_timestamp = true;
    }

    found_timestamp
}

fn exif_u32(exif: &exif::Exif, tag: exif::Tag) -> Option<u32> {
    exif.get_field(tag, exif::In::PRIMARY)
        .and_then(|f| match f.value {
            exif::Value::Long(ref v) => v.first().copied(),
            exif::Value::Short(ref v) => v.first().map(|&x| x as u32),
            _ => None,
        })
}
