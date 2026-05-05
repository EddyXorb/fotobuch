/// Active right-click context menu opened by a short RMB tap.
#[derive(Debug, Clone)]
pub enum ContextMenu {
    Slot {
        page: usize,
        slot: usize,
        screen_pos: egui::Pos2,
    },
    Page {
        page: usize,
        screen_pos: egui::Pos2,
    },
    NavPage {
        page: usize,
        screen_pos: egui::Pos2,
    },
    PoolItem {
        id: String,
        screen_pos: egui::Pos2,
    },
}
