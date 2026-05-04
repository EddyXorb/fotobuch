use std::collections::BTreeSet;

/// Generic multi-selection with single/toggle/range semantics, shared by
/// photo-pool, nav-panel and (internally) slot selection.
#[derive(Default)]
pub enum MultiSelection<K: Ord + Clone> {
    #[default]
    None,
    Active {
        items: BTreeSet<K>,
        anchor: K,
    },
}

impl<K: Ord + Clone> MultiSelection<K> {
    /// Construct from an arbitrary collection; the first element (sorted) becomes the anchor.
    pub fn from_items(items: impl IntoIterator<Item = K>) -> Self {
        let items: BTreeSet<K> = items.into_iter().collect();
        match items.iter().next().cloned() {
            None => Self::None,
            Some(anchor) => Self::Active { items, anchor },
        }
    }

    pub fn single(k: K) -> Self {
        let anchor = k.clone();
        let mut items = BTreeSet::new();
        items.insert(k);
        Self::Active { items, anchor }
    }

    /// Ctrl+click: add or remove.
    pub fn toggle(&mut self, k: K) {
        match self {
            Self::None => *self = Self::single(k),
            Self::Active { items, anchor } => {
                if items.contains(&k) {
                    items.remove(&k);
                    if items.is_empty() {
                        *self = Self::None;
                    } else if *anchor == k {
                        *anchor = items.iter().next().cloned().unwrap_or_else(|| k.clone());
                    }
                } else {
                    *anchor = k.clone();
                    items.insert(k);
                }
            }
        }
    }

    /// Shift+click: extend from anchor to `k` according to `order`.
    pub fn range_to_ordered(&mut self, k: K, order: &[K]) {
        let anchor = match self {
            Self::None => {
                *self = Self::single(k);
                return;
            }
            Self::Active { anchor, .. } => anchor.clone(),
        };
        let pos_a = order.iter().position(|x| x == &anchor);
        let pos_k = order.iter().position(|x| x == &k);
        match (pos_a, pos_k) {
            (std::option::Option::Some(a), std::option::Option::Some(b)) => {
                let items = order[a.min(b)..=a.max(b)].iter().cloned().collect();
                *self = Self::Active { items, anchor };
            }
            _ => *self = Self::single(k),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::None;
    }

    pub fn is_selected(&self, k: &K) -> bool {
        match self {
            Self::None => false,
            Self::Active { items, .. } => items.contains(k),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn items(&self) -> Vec<K> {
        match self {
            Self::None => vec![],
            Self::Active { items, .. } => items.iter().cloned().collect(),
        }
    }
}

impl MultiSelection<usize> {
    /// Shift+click for numeric (index-based) items: range is always lo..=hi.
    pub fn range_to_numeric(&mut self, k: usize) {
        match self {
            Self::None => *self = Self::single(k),
            Self::Active { items, anchor } => {
                let a = *anchor;
                *items = (a.min(k)..=a.max(k)).collect();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_then_toggle_removes() {
        let mut sel: MultiSelection<usize> = MultiSelection::single(2);
        sel.toggle(2);
        assert!(sel.is_empty());
    }

    #[test]
    fn toggle_adds_second_item() {
        let mut sel: MultiSelection<usize> = MultiSelection::single(1);
        sel.toggle(3);
        assert!(sel.is_selected(&1));
        assert!(sel.is_selected(&3));
    }

    #[test]
    fn range_to_numeric_fills_between_anchor_and_target() {
        let mut sel: MultiSelection<usize> = MultiSelection::single(2);
        sel.range_to_numeric(5);
        for i in 2..=5 {
            assert!(sel.is_selected(&i), "missing {i}");
        }
        assert!(!sel.is_selected(&1));
        assert!(!sel.is_selected(&6));
    }

    #[test]
    fn range_to_numeric_backward() {
        let mut sel: MultiSelection<usize> = MultiSelection::single(5);
        sel.range_to_numeric(2);
        for i in 2..=5 {
            assert!(sel.is_selected(&i));
        }
    }

    #[test]
    fn range_to_ordered_string_items() {
        let order: Vec<String> = ["a", "b", "c", "d"].map(String::from).to_vec();
        let mut sel: MultiSelection<String> = MultiSelection::single("b".into());
        sel.range_to_ordered("d".into(), &order);
        assert!(sel.is_selected(&"b".to_string()));
        assert!(sel.is_selected(&"c".to_string()));
        assert!(sel.is_selected(&"d".to_string()));
        assert!(!sel.is_selected(&"a".to_string()));
    }

    #[test]
    fn clear_resets_to_none() {
        let mut sel: MultiSelection<usize> = MultiSelection::single(1);
        sel.clear();
        assert!(sel.is_empty());
        assert_eq!(sel.items(), Vec::<usize>::new());
    }
}
