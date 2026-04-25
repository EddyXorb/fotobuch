use crate::state::{HoveredTarget, InteractionState, SlotSelection};

pub enum PagesForRebuild {
    Selected(Vec<usize>),
    None,
}

pub fn selected_pages_for_rebuild(interaction: &InteractionState) -> PagesForRebuild {
    match &interaction.selections.slots {
        SlotSelection::OnPage { page, .. } => PagesForRebuild::Selected(vec![*page]),
        SlotSelection::None => match &interaction.hovered {
            Some(HoveredTarget::Page { page, .. }) | Some(HoveredTarget::NavPage(page)) => {
                PagesForRebuild::Selected(vec![*page])
            }
            _ => PagesForRebuild::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use fotobuch::dto_models::ProjectState;

    use super::*;
    use crate::state::{GuiState, SlotSelection};

    fn make_interaction() -> crate::state::InteractionState {
        GuiState::new(ProjectState::default()).interaction
    }

    #[test]
    fn selected_pages_for_rebuild_uses_slot_selection_when_present() {
        let mut interaction = make_interaction();
        interaction.selections.slots = SlotSelection::OnPage {
            page: 2,
            slots: BTreeSet::from([0]),
            anchor: 0,
        };
        assert!(
            matches!(selected_pages_for_rebuild(&interaction), PagesForRebuild::Selected(p) if p == vec![2])
        );
    }

    #[test]
    fn selected_pages_for_rebuild_uses_hovered_page_otherwise() {
        let mut interaction = make_interaction();
        interaction.hovered = Some(HoveredTarget::Page {
            page: 5,
            slot: None,
        });
        assert!(
            matches!(selected_pages_for_rebuild(&interaction), PagesForRebuild::Selected(p) if p == vec![5])
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
