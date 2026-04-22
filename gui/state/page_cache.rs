use egui::TextureHandle;

/// GPU-uploaded texture cache for rendered pages and navigation thumbnails.
pub struct PageCache {
    pub textures: Vec<Option<TextureHandle>>,
    pub dirty: Vec<bool>,
    pub thumb_textures: Vec<Option<TextureHandle>>,
}

impl PageCache {
    pub fn new(num_pages: usize) -> Self {
        Self {
            textures: vec![None; num_pages],
            dirty: vec![false; num_pages],
            thumb_textures: vec![None; num_pages],
        }
    }

    pub fn resize(&mut self, new_len: usize) {
        self.textures.resize(new_len, None);
        self.dirty.resize(new_len, false);
        self.thumb_textures.resize(new_len, None);
    }
}
