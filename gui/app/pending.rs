/// Commands that the UI dispatches to the background worker.
///
/// Constructed by `input_handler` and processed by `FotobuchApp::dispatch`.
pub enum PendingCommand {
    Swap {
        src_page: usize,
        src_slot: usize,
        dst_page: usize,
        dst_slot: usize,
    },
    Move {
        src_page: usize,
        src_slot: usize,
        dst_page: usize,
    },
    Undo,
    Redo,
}
