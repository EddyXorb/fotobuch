use std::collections::BTreeSet;

/// Mehrfach-Selektion im Foto-Pool, analog zu [`crate::state::SlotSelection`] für Slots.
#[derive(Default)]
pub enum PhotoSelection {
    #[default]
    None,
    Some {
        ids: BTreeSet<String>,
        anchor: String,
    },
}

impl PhotoSelection {
    pub fn single(id: String) -> Self {
        let anchor = id.clone();
        let mut ids = BTreeSet::new();
        ids.insert(id);
        Self::Some { ids, anchor }
    }

    /// Ctrl+Klick: fügt hinzu oder entfernt.
    pub fn toggle(&mut self, id: String) {
        match self {
            Self::None => {
                *self = Self::single(id);
            }
            Self::Some { ids, anchor } => {
                if ids.contains(&id) {
                    ids.remove(&id);
                    if ids.is_empty() {
                        *self = Self::None;
                    } else if *anchor == id {
                        *anchor = ids.iter().next().cloned().unwrap_or_default();
                    }
                } else {
                    *anchor = id.clone();
                    ids.insert(id);
                }
            }
        }
    }

    /// Shift+Klick: Bereich von Anchor bis `id` (inkl.) gemäß `order`.
    pub fn range_to(&mut self, id: String, order: &[String]) {
        let anchor = match self {
            Self::None => {
                *self = Self::single(id);
                return;
            }
            Self::Some { anchor, .. } => anchor.clone(),
        };

        let pos_anchor = order.iter().position(|s| s == &anchor);
        let pos_id = order.iter().position(|s| s == &id);

        let (start, end) = match (pos_anchor, pos_id) {
            (std::option::Option::Some(a), std::option::Option::Some(b)) => (a.min(b), a.max(b)),
            _ => {
                *self = Self::single(id);
                return;
            }
        };

        let mut ids = BTreeSet::new();
        for item in &order[start..=end] {
            ids.insert(item.clone());
        }
        *self = Self::Some { ids, anchor };
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        *self = Self::None;
    }

    pub fn is_selected(&self, id: &str) -> bool {
        match self {
            Self::None => false,
            Self::Some { ids, .. } => ids.contains(id),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn ids(&self) -> Vec<String> {
        match self {
            Self::None => vec![],
            Self::Some { ids, .. } => ids.iter().cloned().collect(),
        }
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

        // backwards
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
