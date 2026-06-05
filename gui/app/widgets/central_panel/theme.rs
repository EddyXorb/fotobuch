use egui::Color32;

pub struct FbTheme;

impl FbTheme {
    pub const BG: Color32 = Color32::from_rgb(0x1f, 0x20, 0x24);
    pub const STROKE: Color32 = Color32::from_rgb(0x3a, 0x3d, 0x44);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9e, 0xa2, 0xab);
    pub const TEXT_MUTE: Color32 = Color32::from_rgb(0x6b, 0x6f, 0x78);
    pub const ACCENT: Color32 = Color32::from_rgb(0xe0, 0x88, 0x40);
    pub const AUTO: Color32 = Color32::from_rgb(0x6a, 0xa9, 0xff);
    pub const MANUAL: Color32 = Color32::from_rgb(0xc8, 0xb1, 0x8a);
    pub const DANGER: Color32 = Color32::from_rgb(0xd9, 0x77, 0x77);

    pub fn with_alpha(c: Color32, a: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
    }
}
