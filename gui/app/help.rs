use include_dir::{Dir, include_dir};
use std::sync::LazyLock;

static GUI_DOCS_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/docs/book/src/gui");

/// Widget slugs that have no dedicated chapter file → mapped to a real chapter slug.
const SLUG_ALIASES: &[(&str, &str)] = &[
    ("central-hud", "central-panel"),
    ("central-dropzone", "central-panel"),
];

/// (keyword phrase, chapter-slug) — used to build the "Related widgets" chip list.
pub static WIDGET_KEYWORDS: &[(&str, &str)] = &[
    ("Pool panel", "pool-panel"),
    ("page HUD", "central-panel"),
    ("drop zone", "central-panel"),
    ("nav strip", "nav-panel"),
    ("zoom slider", "toolbar"),
    ("toolbar", "toolbar"),
    ("central panel", "central-panel"),
];

pub struct Chapter {
    pub title: String,
    pub slug: String,
    pub text: &'static str,
}

fn collect_md_files(
    dir: &'static include_dir::Dir<'static>,
    out: &mut Vec<&'static include_dir::File<'static>>,
) {
    out.extend(dir.files());
    for sub in dir.dirs() {
        collect_md_files(sub, out);
    }
}

/// All help chapters, auto-discovered recursively from `docs/book/src/gui/`.
/// Title comes from the first `# ` heading; slug from the filename stem.
pub static CHAPTERS: LazyLock<Vec<Chapter>> = LazyLock::new(|| {
    let mut all_files: Vec<&'static include_dir::File<'static>> = Vec::new();
    collect_md_files(&GUI_DOCS_DIR, &mut all_files);

    let mut chapters: Vec<Chapter> = all_files
        .into_iter()
        .filter_map(|f| {
            let path = f.path();
            if path.extension().is_none_or(|e| e != "md") {
                return None;
            }
            let stem = path.file_stem()?.to_str()?;
            let text = f.contents_utf8()?;
            let title = parse_h1_title(text).unwrap_or_else(|| stem_to_title(stem));
            Some(Chapter {
                title,
                slug: stem.to_owned(),
                text,
            })
        })
        .collect();
    chapters.sort_by(|a, b| a.title.cmp(&b.title));
    chapters
});

fn parse_h1_title(text: &str) -> Option<String> {
    text.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l[2..].trim().to_owned())
}

fn stem_to_title(stem: &str) -> String {
    stem.replace('-', " ")
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns the index of the first chapter whose slug matches `slug`, or `None`.
/// Applies `SLUG_ALIASES` for widget slugs that have no dedicated chapter file.
pub fn chapter_for_slug(slug: &str) -> Option<usize> {
    let slug = SLUG_ALIASES
        .iter()
        .find(|(from, _)| *from == slug)
        .map_or(slug, |(_, to)| to);
    CHAPTERS.iter().position(|c| c.slug == slug)
}

/// Returns keyword chips that are relevant to `chapter_slug`.
pub fn chips_for_chapter(chapter_slug: &str) -> impl Iterator<Item = (&'static str, &'static str)> {
    WIDGET_KEYWORDS
        .iter()
        .filter(move |(_, s)| *s == chapter_slug)
        .map(|&(phrase, slug)| (phrase, slug))
}

/// Draw a pulsing golden filled overlay on `rect` when a widget is highlighted.
pub fn draw_glow(painter: &egui::Painter, rect: egui::Rect, time: f64) {
    const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x88, 0x40);
    let t = (time * 3.0).sin() as f32;
    let alpha = ((0.12 + 0.08 * t) * 255.0) as u8;
    let overlay = egui::Color32::from_rgba_unmultiplied(ACCENT.r(), ACCENT.g(), ACCENT.b(), alpha);
    painter.rect_filled(rect, 0.0, overlay);
}
