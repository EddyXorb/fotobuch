/// A floating vertical weight slider opened by the W hotkey over selected slots.
#[derive(Default, Debug, Clone)]
pub enum WeightSlider {
    #[default]
    Closed,
    Open {
        page: usize,
        slots: Vec<usize>,
        screen_pos: egui::Pos2,
        value: f64,
    },
}

impl WeightSlider {
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open { .. })
    }
}
