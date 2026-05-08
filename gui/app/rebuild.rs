use crate::state::InteractionState;

pub enum PagesForRebuild {
    Selected(Vec<usize>),
    None,
}

pub fn selected_pages_for_rebuild(interaction: &InteractionState) -> PagesForRebuild {
    if let Some(page) = interaction.selections.slots.page {
        return PagesForRebuild::Selected(vec![page]);
    }
    let nav = interaction.selections.nav_pages.items();
    if !nav.is_empty() {
        return PagesForRebuild::Selected(nav);
    }
    PagesForRebuild::None
}

#[cfg(test)]
mod tests {
    use fotobuch::dto_models::ProjectState;

    use super::*;
    use crate::state::{GuiState, SlotSelection};

    fn make_interaction() -> crate::state::InteractionState {
        GuiState::new(ProjectState::default(), Default::default(), vec![], false).interaction
    }

    #[test]
    fn selected_pages_for_rebuild_uses_slot_selection_when_present() {
        let mut interaction = make_interaction();
        interaction.selections.slots = SlotSelection::single(2, 0);
        assert!(
            matches!(selected_pages_for_rebuild(&interaction), PagesForRebuild::Selected(p) if p == vec![2])
        );
    }

    #[test]
    fn selected_pages_for_rebuild_uses_nav_pages_when_no_slot_sel() {
        let mut interaction = make_interaction();
        interaction.selections.nav_pages = crate::state::MultiSelection::single(3);
        assert!(
            matches!(selected_pages_for_rebuild(&interaction), PagesForRebuild::Selected(p) if p == vec![3])
        );
    }

    #[test]
    fn selected_pages_for_rebuild_returns_none_without_signal() {
        let interaction = make_interaction();
        assert!(matches!(
            selected_pages_for_rebuild(&interaction),
            PagesForRebuild::None
        ));
    }
}
