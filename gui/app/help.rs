#[allow(dead_code)]
pub const HELP_SLUGS: &[&str] = &[
    "panel-pool",
    "panel-nav",
    "toolbar",
    "central-hud",
    "central-dropzone",
    "central-page",
];

/// (keyword phrase, slug) — used to build the "Related widgets" chip list.
pub static WIDGET_KEYWORDS: &[(&str, &str)] = &[
    ("Pool panel", "panel-pool"),
    ("page HUD", "central-hud"),
    ("drop zone", "central-dropzone"),
    ("nav strip", "panel-nav"),
    ("zoom slider", "toolbar"),
    ("toolbar", "toolbar"),
    ("central panel", "central-page"),
];

pub struct Chapter {
    pub title: &'static str,
    pub slug: &'static str,
    pub text: &'static str,
}

pub static CHAPTERS: &[Chapter] = &[
    Chapter {
        title: "Overview",
        slug: "overview",
        text: include_str!("../../docs/book/src/gui/overview.md"),
    },
    Chapter {
        title: "Central panel",
        slug: "central-page",
        text: include_str!("../../docs/book/src/gui/central-panel.md"),
    },
    Chapter {
        title: "Pool panel",
        slug: "panel-pool",
        text: include_str!("../../docs/book/src/gui/pool-panel.md"),
    },
    Chapter {
        title: "Toolbar",
        slug: "toolbar",
        text: include_str!("../../docs/book/src/gui/toolbar.md"),
    },
    Chapter {
        title: "Nav strip",
        slug: "panel-nav",
        text: include_str!("../../docs/book/src/gui/nav-panel.md"),
    },
    Chapter {
        title: "Keyboard",
        slug: "keyboard",
        text: include_str!("../../docs/book/src/gui/keyboard.md"),
    },
];

/// Returns the index of the first chapter whose slug matches `slug`, or `None`.
pub fn chapter_for_slug(slug: &str) -> Option<usize> {
    CHAPTERS.iter().position(|c| c.slug == slug)
}

/// Returns keyword chips that are relevant to `chapter_slug`.
pub fn chips_for_chapter(chapter_slug: &str) -> impl Iterator<Item = (&'static str, &'static str)> {
    WIDGET_KEYWORDS
        .iter()
        .filter(move |(_, s)| *s == chapter_slug)
        .map(|&(phrase, slug)| (phrase, slug))
}

/// Draw a pulsing golden glow ring around `rect` when a widget is highlighted.
pub fn draw_glow(painter: &egui::Painter, rect: egui::Rect, time: f64) {
    const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x88, 0x40);
    let t = (time * 5.0).sin() as f32;
    let outer_alpha = ((0.25 + 0.15 * t) * 255.0) as u8;
    let inner_alpha = ((0.45 + 0.20 * t) * 255.0) as u8;
    let outer_color =
        egui::Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), outer_alpha);
    let inner_color =
        egui::Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), inner_alpha);
    painter.rect_stroke(
        rect.expand(4.0),
        4.0,
        egui::Stroke::new(2.0, outer_color),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        rect.expand(1.5),
        2.0,
        egui::Stroke::new(1.5, inner_color),
        egui::StrokeKind::Outside,
    );
}
