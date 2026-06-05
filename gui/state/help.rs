use egui_commonmark::CommonMarkCache;

#[derive(Default)]
pub struct HelpState {
    pub open: bool,
    pub lens_active: bool,
    /// Slug + screen rect of the documentable widget last under the pointer this frame.
    /// Reset to `None` at frame start; written unconditionally by each documentable widget.
    pub hovered_widget: Option<(&'static str, egui::Rect)>,
    /// Slug highlighted by a keyword chip click.  Cleared when the window closes.
    pub highlighted: Option<&'static str>,
    /// Active chapter index in the help window sidebar.
    pub chapter: usize,
    pub cache: CommonMarkCache,
}
