use std::path::PathBuf;

#[derive(Default)]
pub struct AddDialogState {
    pub open: bool,
    /// Selected paths (from native file picker or OS drop).
    pub pending_paths: Vec<PathBuf>,
    pub recursive: bool,
    pub weight_buffer: String,
    pub source_filter: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dialog_submit_consumes_pending_paths() {
        let mut state = AddDialogState {
            open: true,
            pending_paths: vec![PathBuf::from("/some/dir")],
            recursive: false,
            weight_buffer: "2.0".to_string(),
            source_filter: String::new(),
        };
        let paths = std::mem::take(&mut state.pending_paths);
        state.open = false;
        assert!(paths.len() == 1);
        assert!(state.pending_paths.is_empty());
        assert!(!state.open);
        assert_eq!(paths[0], PathBuf::from("/some/dir"));
    }
}
