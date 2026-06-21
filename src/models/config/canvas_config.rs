pub trait CanvasConfig {
    fn page_width_mm(&self) -> f64;
    fn page_height_mm(&self) -> f64;
    fn bleed_mm(&self) -> f64;
    fn margin_mm(&self) -> f64;
    fn gap_mm(&self) -> f64;
    fn bleed_threshold_mm(&self) -> f64;
}
