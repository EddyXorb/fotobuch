use super::MultiSelection;

/// Mehrfach-Selektion im Foto-Pool.
pub struct PhotoSelection(pub MultiSelection<String>);

impl Default for PhotoSelection {
    fn default() -> Self {
        Self(MultiSelection::None)
    }
}

impl PhotoSelection {
    pub fn single(id: String) -> Self {
        Self(MultiSelection::single(id))
    }

    pub fn toggle(&mut self, id: String) {
        self.0.toggle(id);
    }

    pub fn range_to(&mut self, id: String, order: &[String]) {
        self.0.range_to_ordered(id, order);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn select_all(&mut self, ids: impl IntoIterator<Item = String>) {
        self.0 = super::MultiSelection::from_items(ids);
    }

    pub fn is_selected(&self, id: &str) -> bool {
        self.0.is_selected(&id.to_owned())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn ids(&self) -> Vec<String> {
        self.0.items()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_replaces_previous() {
        let mut sel = PhotoSelection::single("a".into());
        sel = PhotoSelection::single("b".into());
        assert!(sel.is_selected("b"));
        assert!(!sel.is_selected("a"));
    }

    #[test]
    fn toggle_adds_then_removes() {
        let mut sel = PhotoSelection::single("a".into());
        sel.toggle("b".into());
        assert!(sel.is_selected("a"));
        assert!(sel.is_selected("b"));
        sel.toggle("a".into());
        assert!(!sel.is_selected("a"));
        assert!(sel.is_selected("b"));
    }

    #[test]
    fn range_fills_forward_and_backward() {
        let order: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut sel = PhotoSelection::single("b".into());
        sel.range_to("d".into(), &order);
        assert!(sel.is_selected("b"));
        assert!(sel.is_selected("c"));
        assert!(sel.is_selected("d"));
        assert!(!sel.is_selected("a"));
        assert!(!sel.is_selected("e"));

        let mut sel2 = PhotoSelection::single("d".into());
        sel2.range_to("b".into(), &order);
        assert!(sel2.is_selected("b"));
        assert!(sel2.is_selected("c"));
        assert!(sel2.is_selected("d"));
    }

    #[test]
    fn clear_resets() {
        let mut sel = PhotoSelection::single("a".into());
        sel.clear();
        assert!(sel.is_empty());
    }
}
