//! Render-level regression tests for the spine text position on the cover.
//!
//! The cover is rasterised and the horizontal centre of all dark pixels
//! (= the rotated spine text; no photos, borders or watermark are drawn) is
//! compared with the centre of the cover canvas.

use anyhow::Result;
use fotobuch::commands::project::new::{NewConfig, new};
use fotobuch::models::LayoutPage;
use fotobuch::output::typst::{TypstWorld, render_pages_with_world};
use fotobuch::state_manager::StateManager;
use tempfile::TempDir;

const MM_PER_PT: f64 = 25.4 / 72.0;
const PIXEL_PER_PT: f32 = 2.0;
const COVER_BLEED_MM: f64 = 3.0;

/// Creates a cover project with 20 empty inner pages and returns the
/// (horizontal, vertical) offset in mm of the spine text centre from the
/// canvas centre.
fn spine_text_offset_mm(spine_grow: Option<f64>, spine_mm: Option<f64>) -> Result<(f64, f64)> {
    let tmp = TempDir::new()?;
    let config = NewConfig {
        name: "spinebook".to_string(),
        width_mm: 200.0,
        height_mm: 200.0,
        bleed_mm: COVER_BLEED_MM,
        with_cover: true,
        cover_width_mm: Some(400.0),
        cover_height_mm: Some(200.0),
        spine_grow_per_10_pages_mm: spine_grow,
        spine_mm,
        margin_mm: 0.0,
        base_config: None,
    };
    let root = new(tmp.path(), &config)?.result.project_root;

    let mut mgr = StateManager::open(&root)?;
    mgr.state.config.book.cover.spine_text = Some("SPINE".to_string());
    mgr.state.config.preview.show_borders = false;
    mgr.state.config.preview.show_preview_watermark = false;
    // cover + 20 empty inner pages
    mgr.state.layout = (0..21)
        .map(|_| LayoutPage {
            photos: vec![],
            slots: vec![],
            mode: Default::default(),
        })
        .collect();
    mgr.finish("test: spine render setup")?;

    let mut world = TypstWorld::new(&root, "spinebook")?;
    let page = render_pages_with_world(&mut world, &[0], PIXEL_PER_PT)?
        .into_iter()
        .next()
        .expect("cover page rendered");

    let (mut sum_x, mut sum_y, mut count) = (0u64, 0u64, 0u64);
    for (i, px) in page.pixels.as_chunks::<4>().0.iter().enumerate() {
        let dark = px[3] > 128 && px[0] < 100 && px[1] < 100 && px[2] < 100;
        if dark {
            sum_x += u64::from(i as u32 % page.width);
            sum_y += u64::from(i as u32 / page.width);
            count += 1;
        }
    }
    assert!(count > 0, "no spine text found on the cover");

    let px_to_mm = MM_PER_PT / f64::from(PIXEL_PER_PT);
    let centre_offset =
        |sum: u64, extent: u32| (sum as f64 / count as f64 - f64::from(extent) / 2.0) * px_to_mm;
    Ok((
        centre_offset(sum_x, page.width),
        centre_offset(sum_y, page.height),
    ))
}

#[test]
fn spine_text_is_centred_in_auto_spine_mode() -> Result<()> {
    // 20 inner pages × 10 mm / 10 pages = 20 mm spine, canvas = 400 + 20 mm
    let (offset, _) = spine_text_offset_mm(Some(10.0), None)?;
    assert!(
        offset.abs() < 1.0,
        "spine text is {offset:.2} mm off the spine centre"
    );
    Ok(())
}

#[test]
fn spine_text_is_centred_in_fixed_spine_mode() -> Result<()> {
    // fixed: canvas width already includes the spine
    let (offset, _) = spine_text_offset_mm(None, Some(20.0))?;
    assert!(
        offset.abs() < 1.0,
        "spine text is {offset:.2} mm off the spine centre"
    );
    Ok(())
}

#[test]
fn spine_text_is_centred_along_the_spine() -> Result<()> {
    let (_, offset) = spine_text_offset_mm(Some(10.0), None)?;
    assert!(
        offset.abs() < 1.0,
        "spine text is {offset:.2} mm off the vertical cover centre"
    );
    Ok(())
}
