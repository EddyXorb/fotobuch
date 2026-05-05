/// UI state for the "New Project" dialog.
#[derive(Default)]
#[allow(dead_code)]
pub struct NewProjectDialogState {
    pub open: bool,
    pub name: String,
    pub width_mm: String,
    pub height_mm: String,
    pub bleed_mm: String,
    pub margin_mm: String,
    pub with_cover: bool,
    /// Spine mode: false = auto (mm per 10 pages), true = fixed mm
    pub spine_fixed: bool,
    pub spine_value: String,
}

impl NewProjectDialogState {
    pub fn reset(&mut self) {
        *self = Self {
            open: self.open,
            width_mm: "210".to_string(),
            height_mm: "297".to_string(),
            bleed_mm: "3".to_string(),
            margin_mm: "10".to_string(),
            ..Default::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_keeps_open_flag() {
        let mut s = NewProjectDialogState::default();
        s.open = true;
        s.name = "old".to_string();
        s.reset();
        assert!(s.open);
        assert!(s.name.is_empty());
        assert_eq!(s.width_mm, "210");
    }
}
