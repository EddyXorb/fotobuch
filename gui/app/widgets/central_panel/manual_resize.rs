/// Compute the new (x, y, w, h) in mm after a SE-corner drag.
///
/// SE-resize keeps the NW corner (x, y) fixed and adjusts (w, h) proportionally
/// to maintain the aspect ratio, using the diagonal distance from NW to new SE.
pub fn compute_se(
    origin: (f64, f64, f64, f64), // x, y, w, h in mm
    delta_px: egui::Vec2,
    pixel_per_mm: f64,
) -> (f64, f64, f64, f64) {
    let (x, y, w, h) = origin;
    let orig_diag = (w * w + h * h).sqrt();
    if orig_diag < f64::EPSILON || pixel_per_mm < f64::EPSILON {
        return origin;
    }

    let dx_mm = delta_px.x as f64 / pixel_per_mm;
    let dy_mm = delta_px.y as f64 / pixel_per_mm;

    // New SE position relative to NW corner.
    let new_se_x = w + dx_mm;
    let new_se_y = h + dy_mm;
    let new_diag = (new_se_x * new_se_x + new_se_y * new_se_y).sqrt();

    let scale = new_diag / orig_diag;
    let min_dim = 1.0_f64; // mm
    let new_w = (w * scale).max(min_dim);
    let new_h = (h * scale).max(min_dim);

    (x, y, new_w, new_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_se_zero_delta_is_identity() {
        let orig = (10.0, 20.0, 80.0, 60.0);
        let result = compute_se(orig, egui::Vec2::ZERO, 3.0);
        let eps = 1e-9;
        assert!((result.0 - orig.0).abs() < eps);
        assert!((result.1 - orig.1).abs() < eps);
        assert!((result.2 - orig.2).abs() < eps);
        assert!((result.3 - orig.3).abs() < eps);
    }

    #[test]
    fn compute_se_keeps_origin_xy() {
        let orig = (5.0, 15.0, 100.0, 50.0);
        let result = compute_se(orig, egui::vec2(30.0, 20.0), 5.0);
        let eps = 1e-9;
        assert!((result.0 - orig.0).abs() < eps, "x must stay fixed");
        assert!((result.1 - orig.1).abs() < eps, "y must stay fixed");
    }

    #[test]
    fn compute_se_keeps_aspect_ratio() {
        let (x, y, w, h) = (0.0, 0.0, 100.0, 50.0);
        let result = compute_se((x, y, w, h), egui::vec2(50.0, 25.0), 5.0);
        // aspect ratio = w/h; new should be close
        let orig_ratio = w / h;
        let new_ratio = result.2 / result.3;
        assert!(
            (new_ratio - orig_ratio).abs() < 0.01,
            "aspect ratio must be preserved: {new_ratio} vs {orig_ratio}"
        );
    }

    #[test]
    fn compute_se_negative_delta_shrinks() {
        let orig = (0.0, 0.0, 100.0, 50.0);
        let result = compute_se(orig, egui::vec2(-20.0, -10.0), 1.0);
        assert!(result.2 < orig.2, "width should shrink");
        assert!(result.3 < orig.3, "height should shrink");
    }
}
