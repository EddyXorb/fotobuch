use std::collections::HashMap;

#[derive(Default)]
pub struct ConfigPanelState {
    pub open: bool,
    /// Bearbeitungspuffer pro Pfad (dot-notation key → rohe String-Eingabe).
    pub edit_buffers: HashMap<String, String>,
}
