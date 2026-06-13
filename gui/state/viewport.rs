/// Scroll/zoom state for the central panel.
#[derive(Default)]
pub struct ScrollState {
    pub scroll_y: f32,
    pub viewport_top: f32,
    pub pending_scroll_y: Option<f32>,
    /// Easing target for explicit navigation (nav click, Home/End, goto).
    /// Interpolated toward scroll_y each frame at 25% step.
    pub ease_target: Option<f32>,
}

/// A brief highlight effect on a slot in the central panel, triggered when a
/// placed pool photo is clicked, to draw the eye after auto-scrolling to it.
pub struct SlotFlash {
    pub page: usize,
    pub slot: usize,
    /// Context time (seconds) at which the flash started.
    pub start: f64,
}

/// Total duration of the slot flash effect in seconds.
pub const FLASH_DURATION: f64 = 1.8;

/// Flash overlay intensity in `0.0..=1.0` for `elapsed` seconds since the flash
/// started, or `None` once the effect has finished. Pulses while fading out.
pub fn flash_intensity(elapsed: f64, duration: f64) -> Option<f32> {
    if elapsed < 0.0 || elapsed >= duration {
        return None;
    }
    let fade = 1.0 - (elapsed / duration) as f32;
    let pulse = 0.5 + 0.5 * (elapsed * 9.0).sin() as f32;
    Some(fade * pulse)
}

/// View parameters for the central panel: zoom level, base DPI scale, scroll, and nav scroll target.
pub struct Viewport {
    pub zoom: f32,
    pub pixel_per_pt: f32,
    pub scroll: ScrollState,
    pub scroll_to_page: Option<usize>,
    /// When `true`, zoom will be adjusted on the next frame to fit the widest page.
    pub fit_pending: bool,
    /// Active slot flash effect, if any.
    pub flash: Option<SlotFlash>,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pixel_per_pt: 1.5,
            scroll: ScrollState::default(),
            scroll_to_page: None,
            fit_pending: true,
            flash: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_intensity_is_none_outside_the_effect_window() {
        assert_eq!(flash_intensity(-0.1, FLASH_DURATION), None);
        assert_eq!(flash_intensity(FLASH_DURATION, FLASH_DURATION), None);
        assert_eq!(flash_intensity(FLASH_DURATION + 1.0, FLASH_DURATION), None);
    }

    #[test]
    fn flash_intensity_stays_within_unit_range_and_fades_to_zero() {
        let mut t = 0.0;
        while t < FLASH_DURATION {
            let v = flash_intensity(t, FLASH_DURATION).unwrap();
            assert!(
                (0.0..=1.0).contains(&v),
                "intensity {v} out of range at {t}"
            );
            t += 0.05;
        }
        // Near the end the fade envelope has driven the effect close to zero.
        let late = flash_intensity(FLASH_DURATION - 0.01, FLASH_DURATION).unwrap();
        assert!(
            late < 0.1,
            "expected near-zero intensity at the end, got {late}"
        );
    }
}
