use std::path::Path;

use anyhow::Result;

/// Lädt ein Bild, dekodiert es und skaliert es auf `max_edge` px (längste Kante).
pub fn load(path: &Path, max_edge: u32) -> Result<(u32, u32, Vec<u8>)> {
    let img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .thumbnail(max_edge, max_edge);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}
