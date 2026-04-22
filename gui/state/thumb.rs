use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use egui::TextureHandle;

#[derive(Default)]
pub struct PhotoThumbState {
    pub thumbs: HashMap<String, TextureHandle>,
    pub in_flight: HashSet<String>,
    pub prefetch: VecDeque<String>,
    pub pending_loads: Vec<(String, PathBuf)>,
}
