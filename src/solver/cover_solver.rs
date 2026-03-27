//! Deterministic cover slot calculator.
//!
//! For cover modes other than [`CoverMode::Free`], this module computes fixed
//! slot positions from the cover geometry — no GA solver is involved.
//!
//! ## Coordinate system
//!
//! All returned [`Slot`] coordinates are in **canvas space**: origin at the
//! top-left of the content area after the margin has been subtracted on all
//! sides.  This matches what the GA solver produces and what the Typst template
//! expects.
//!
//! ## Panel geometry (auto spine mode)
//!
//! ```text
//! canvas x = 0                                          canvas x = canvas_w
//! |<--- back (half_fb − margin) --->|<-- spine -->|<--- front (half_fb − margin) --->|
//! ```
//!
//! where `half_fb = front_back_width_mm / 2`.  `spine_clearance_mm` is
//! subtracted from the photo area on both sides of the spine.
//!
//! In **fixed** spine mode the canvas width equals `front_back_width_mm` and
//! the spine is centred at `canvas_w / 2`.

use anyhow::{Result, bail};
use tracing::warn;

use crate::dto_models::{CoverConfig, CoverMode, Slot};

// ── public entry point ───────────────────────────────────────────────────────

/// Compute cover slots deterministically for a non-`Free` cover mode.
///
/// `photo_ratios` must contain one `width/height` ratio per photo currently
/// assigned to page 0, in the same order as `layout[0].photos`.
///
/// Returns a slot for every entry in `photo_ratios`.  The caller is responsible
/// for checking that `photo_ratios.len()` matches
/// [`CoverMode::required_slots`] (see [`warn_slot_count_mismatch`]).
///
/// # Errors
///
/// Returns an error when the mode is `Free` (caller should use the GA instead)
/// or when the cover dimensions are zero / degenerate.
pub fn compute_cover_slots(
    cover: &CoverConfig,
    photo_ratios: &[f64],
    inner_page_count: usize,
) -> Result<Vec<Slot>> {
    if cover.mode.is_free() {
        bail!("compute_cover_slots called for Free mode — use the GA solver instead");
    }

    let areas = cover_areas(cover, inner_page_count)?;

    let slots = match cover.mode {
        CoverMode::Free => unreachable!(),

        CoverMode::Front => vec![fit_in(&areas.front, photo_ratio(photo_ratios, 0)?)],
        CoverMode::FrontFull => vec![fill(&areas.front)],

        CoverMode::Back => vec![fit_in(&areas.back, photo_ratio(photo_ratios, 0)?)],
        CoverMode::BackFull => vec![fill(&areas.back)],

        CoverMode::Spread => vec![fit_in(&areas.spread, photo_ratio(photo_ratios, 0)?)],
        CoverMode::SpreadFull => vec![fill(&areas.spread)],

        CoverMode::Split => vec![
            fit_in(&areas.front, photo_ratio(photo_ratios, 0)?),
            fit_in(&areas.back, photo_ratio(photo_ratios, 1)?),
        ],
        CoverMode::SplitFull => vec![fill(&areas.front), fill(&areas.back)],
    };

    Ok(slots)
}

/// Emit a `warn!` when the number of photos on the cover does not match what
/// the selected mode expects.  Should be called before [`compute_cover_slots`].
pub fn warn_slot_count_mismatch(mode: CoverMode, photo_count: usize) {
    let Some(expected) = mode.required_slots() else {
        return; // Free mode: any count is fine
    };
    if photo_count != expected {
        warn!(
            "Cover mode `{mode:?}` expects {expected} photo(s), \
             but page 0 has {photo_count}. \
             Use `fotobuch place <photo> --into 0` to assign the right photos, \
             then `fotobuch rebuild --page 0`."
        );
    }
}

// ── internal geometry ────────────────────────────────────────────────────────

/// Axis-aligned rectangle in canvas coordinates.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// All panel areas in canvas coordinates for one cover configuration.
struct CoverAreas {
    front: Rect,
    back: Rect,
    /// Full spread — used only for spread/spread-full modes.
    spread: Rect,
}

/// Compute the front, back, and spread rectangles in canvas coordinates.
fn cover_areas(cover: &CoverConfig, inner_page_count: usize) -> Result<CoverAreas> {
    let margin = cover.margin_mm;
    let clearance = cover.spine_clearance_mm;
    let canvas_h = cover.height_mm - 2.0 * margin;
    let half_fb = cover.front_back_width_mm / 2.0;
    let spine_w = cover.spine_width_mm(inner_page_count);

    if canvas_h <= 0.0 || cover.front_back_width_mm <= 0.0 {
        bail!(
            "Cover dimensions are zero or negative (height_mm={}, front_back_width_mm={}). \
             Please set valid cover dimensions in the config.",
            cover.height_mm,
            cover.front_back_width_mm,
        );
    }

    let (back, front) = match &cover.spine {
        // Canvas width = front_back + spine. The spine sits between the two panels.
        crate::dto_models::SpineConfig::Auto { .. } => {
            let canvas_w = cover.front_back_width_mm + spine_w - 2.0 * margin;
            let spine_start = half_fb - margin; // canvas-x where spine begins
            let back = Rect {
                x: 0.0,
                y: 0.0,
                w: (spine_start - clearance).max(0.0),
                h: canvas_h,
            };
            let front = Rect {
                x: (spine_start + spine_w + clearance).min(canvas_w),
                y: 0.0,
                w: (canvas_w - (spine_start + spine_w + clearance)).max(0.0),
                h: canvas_h,
            };
            (back, front)
        }

        // Canvas width = front_back only. Spine is a visual overlay at the centre.
        crate::dto_models::SpineConfig::Fixed { .. } => {
            let canvas_w = cover.front_back_width_mm - 2.0 * margin;
            let spine_center = canvas_w / 2.0;
            let spine_half = spine_w / 2.0;
            let back = Rect {
                x: 0.0,
                y: 0.0,
                w: (spine_center - spine_half - clearance).max(0.0),
                h: canvas_h,
            };
            let front = Rect {
                x: (spine_center + spine_half + clearance).min(canvas_w),
                y: 0.0,
                w: (canvas_w - (spine_center + spine_half + clearance)).max(0.0),
                h: canvas_h,
            };
            (back, front)
        }
    };

    // Spread covers the full canvas; no spine avoidance.
    let spread_w = match &cover.spine {
        crate::dto_models::SpineConfig::Auto { .. } => {
            cover.front_back_width_mm + spine_w - 2.0 * margin
        }
        crate::dto_models::SpineConfig::Fixed { .. } => cover.front_back_width_mm - 2.0 * margin,
    };
    let spread = Rect {
        x: 0.0,
        y: 0.0,
        w: spread_w,
        h: canvas_h,
    };

    Ok(CoverAreas {
        front,
        back,
        spread,
    })
}

// ── slot construction helpers ────────────────────────────────────────────────

/// Fit photo (aspect ratio `ratio = w/h`) inside `area`, preserving ratio, centred.
fn fit_in(area: &Rect, ratio: f64) -> Slot {
    let area_ratio = area.w / area.h;
    let (w, h) = if ratio >= area_ratio {
        // photo wider than area → clamp width
        (area.w, area.w / ratio)
    } else {
        // photo taller than area → clamp height
        (area.h * ratio, area.h)
    };
    Slot {
        x_mm: area.x + (area.w - w) / 2.0,
        y_mm: area.y + (area.h - h) / 2.0,
        width_mm: w,
        height_mm: h,
    }
}

/// Slot that fills `area` exactly (photo will be cropped by the template).
fn fill(area: &Rect) -> Slot {
    Slot {
        x_mm: area.x,
        y_mm: area.y,
        width_mm: area.w,
        height_mm: area.h,
    }
}

/// Extract the aspect ratio at `index` from the slice, or error with a clear message.
fn photo_ratio(ratios: &[f64], index: usize) -> Result<f64> {
    ratios.get(index).copied().ok_or_else(|| {
        anyhow::anyhow!(
            "Cover solver expected a photo at index {index} but only {} photo(s) are on the cover. \
             Assign the correct number of photos with `fotobuch place <photo> --into 0`.",
            ratios.len()
        )
    })
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto_models::{CoverMode, SpineConfig};

    fn base_cover(mode: CoverMode) -> CoverConfig {
        CoverConfig {
            active: true,
            mode,
            spine_clearance_mm: 5.0,
            spine: SpineConfig::Auto {
                spine_mm_per_10_pages: 1.4,
            },
            front_back_width_mm: 420.0, // 2 × 210 mm
            height_mm: 297.0,
            spine_text: None,
            bleed_mm: 3.0,
            margin_mm: 0.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }

    /// helper: spine_w for 10 pages with 1.4 mm/10 pages = 1.4 mm
    const INNER: usize = 10;
    const SPINE: f64 = 1.4; // mm
    const HALF_FB: f64 = 210.0; // front_back / 2
    const CLEARANCE: f64 = 5.0;
    const H: f64 = 297.0;

    // ── front ────────────────────────────────────────────────────────────────

    #[test]
    fn front_full_fills_front_panel() {
        let cover = base_cover(CoverMode::FrontFull);
        let slots = compute_cover_slots(&cover, &[1.5], INNER).unwrap();
        assert_eq!(slots.len(), 1);
        let s = &slots[0];
        // front panel starts at half_fb + spine_w, no clearance for "full" modes
        assert!((s.x_mm - (HALF_FB + SPINE + CLEARANCE)).abs() < 1e-6);
        assert!((s.y_mm).abs() < 1e-6);
        assert!((s.width_mm - (HALF_FB - CLEARANCE)).abs() < 1e-6);
        assert!((s.height_mm - H).abs() < 1e-6);
    }

    #[test]
    fn front_fit_preserves_aspect_ratio() {
        let cover = base_cover(CoverMode::Front);
        // landscape photo 3:2 in a portrait-ish panel (205 × 297)
        let slots = compute_cover_slots(&cover, &[3.0 / 2.0], INNER).unwrap();
        let s = &slots[0];
        let ratio = s.width_mm / s.height_mm;
        assert!((ratio - 1.5).abs() < 1e-4, "ratio={ratio}");
        // must fit within front panel
        let front_x = HALF_FB + SPINE + CLEARANCE;
        let front_w = HALF_FB - CLEARANCE;
        assert!(s.x_mm >= front_x - 1e-6);
        assert!(s.x_mm + s.width_mm <= front_x + front_w + 1e-6);
        assert!(s.y_mm >= -1e-6);
        assert!(s.y_mm + s.height_mm <= H + 1e-6);
    }

    // ── back ─────────────────────────────────────────────────────────────────

    #[test]
    fn back_full_fills_back_panel() {
        let cover = base_cover(CoverMode::BackFull);
        let slots = compute_cover_slots(&cover, &[1.0], INNER).unwrap();
        assert_eq!(slots.len(), 1);
        let s = &slots[0];
        assert!((s.x_mm).abs() < 1e-6);
        assert!((s.y_mm).abs() < 1e-6);
        assert!((s.width_mm - (HALF_FB - CLEARANCE)).abs() < 1e-6);
        assert!((s.height_mm - H).abs() < 1e-6);
    }

    // ── spread ───────────────────────────────────────────────────────────────

    #[test]
    fn spread_full_covers_entire_canvas() {
        let cover = base_cover(CoverMode::SpreadFull);
        let slots = compute_cover_slots(&cover, &[2.0], INNER).unwrap();
        assert_eq!(slots.len(), 1);
        let s = &slots[0];
        let canvas_w = 420.0 + SPINE;
        assert!((s.x_mm).abs() < 1e-6);
        assert!((s.y_mm).abs() < 1e-6);
        assert!((s.width_mm - canvas_w).abs() < 1e-6);
        assert!((s.height_mm - H).abs() < 1e-6);
    }

    #[test]
    fn spread_fit_uses_full_canvas_no_clearance() {
        let cover = base_cover(CoverMode::Spread);
        let ratio = 3.0; // very wide
        let slots = compute_cover_slots(&cover, &[ratio], INNER).unwrap();
        let s = &slots[0];
        let canvas_w = 420.0 + SPINE;
        // must fit within full canvas
        assert!(s.x_mm >= -1e-6);
        assert!(s.x_mm + s.width_mm <= canvas_w + 1e-6);
        let out_ratio = s.width_mm / s.height_mm;
        assert!((out_ratio - ratio).abs() < 1e-4);
    }

    // ── split ────────────────────────────────────────────────────────────────

    #[test]
    fn split_full_two_slots() {
        let cover = base_cover(CoverMode::SplitFull);
        let slots = compute_cover_slots(&cover, &[1.0, 1.0], INNER).unwrap();
        assert_eq!(slots.len(), 2);
        // slot 0 = front
        let front_x = HALF_FB + SPINE + CLEARANCE;
        assert!((slots[0].x_mm - front_x).abs() < 1e-6);
        // slot 1 = back
        assert!((slots[1].x_mm).abs() < 1e-6);
    }

    #[test]
    fn split_fit_two_slots_preserve_ratio() {
        let cover = base_cover(CoverMode::Split);
        let r0 = 1.5_f64;
        let r1 = 0.75_f64;
        let slots = compute_cover_slots(&cover, &[r0, r1], INNER).unwrap();
        assert_eq!(slots.len(), 2);
        assert!((slots[0].width_mm / slots[0].height_mm - r0).abs() < 1e-4);
        assert!((slots[1].width_mm / slots[1].height_mm - r1).abs() < 1e-4);
    }

    // ── error cases ──────────────────────────────────────────────────────────

    #[test]
    fn free_mode_returns_error() {
        let cover = base_cover(CoverMode::Free);
        assert!(compute_cover_slots(&cover, &[1.0], INNER).is_err());
    }

    #[test]
    fn missing_photo_returns_error() {
        let cover = base_cover(CoverMode::Split); // needs 2
        assert!(compute_cover_slots(&cover, &[1.5], INNER).is_err());
    }

    #[test]
    fn zero_dimensions_returns_error() {
        let mut cover = base_cover(CoverMode::Front);
        cover.height_mm = 0.0;
        assert!(compute_cover_slots(&cover, &[1.5], INNER).is_err());
    }

    // ── fixed spine ──────────────────────────────────────────────────────────

    #[test]
    fn fixed_spine_front_full() {
        let mut cover = base_cover(CoverMode::FrontFull);
        cover.spine = SpineConfig::Fixed {
            spine_width_mm: 4.0,
        };
        let slots = compute_cover_slots(&cover, &[1.0], INNER).unwrap();
        let s = &slots[0];
        // canvas_w = 420, center = 210, spine_half = 2, front_x = 217
        let expected_x = 210.0 + 2.0 + CLEARANCE;
        assert!((s.x_mm - expected_x).abs() < 1e-6, "x={}", s.x_mm);
    }

    // ── margin taken into account ─────────────────────────────────────────────

    #[test]
    fn margin_shifts_panels() {
        let mut cover = base_cover(CoverMode::FrontFull);
        cover.margin_mm = 10.0;
        let slots = compute_cover_slots(&cover, &[1.5], INNER).unwrap();
        let s = &slots[0];
        // With margin=10: canvas_w = 420+1.4-20 = 401.4
        // spine_start (canvas) = 210 - 10 = 200
        // front_x = 200 + 1.4 + 5 = 206.4
        let spine_start = HALF_FB - 10.0;
        let expected_x = spine_start + SPINE + CLEARANCE;
        assert!((s.x_mm - expected_x).abs() < 1e-6, "x={}", s.x_mm);
        // canvas_h = 297 - 20 = 277
        assert!((s.height_mm - (H - 20.0)).abs() < 1e-6);
    }

    // ── symmetry: front_w == back_w ───────────────────────────────────────────

    #[test]
    fn front_and_back_widths_are_symmetric() {
        let cover = base_cover(CoverMode::SplitFull);
        let slots = compute_cover_slots(&cover, &[1.0, 1.0], INNER).unwrap();
        assert!(
            (slots[0].width_mm - slots[1].width_mm).abs() < 1e-6,
            "front_w={} back_w={}",
            slots[0].width_mm,
            slots[1].width_mm
        );
    }
}
