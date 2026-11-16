//! Template substitution and generation

use anyhow::Result;

/// The base Typst template with {name} placeholders
const TEMPLATE_BASE: &str = include_str!("../../../templates/fotobuch.typ");

/// Generate a Typst template for a specific project
pub fn generate_template(project_name: &str) -> Result<String> {
    validate_template_name(project_name)?;
    Ok(TEMPLATE_BASE.replace("{project_name}", project_name))
}

/// Validate that a project name is safe for template substitution
fn validate_template_name(name: &str) -> Result<()> {
    // This should match the project name validation
    // but we double-check here for safety
    if name.contains("..") {
        anyhow::bail!("Project name cannot contain '..'");
    }
    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("Project name cannot contain path separators");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_template() {
        let template = generate_template("myproject").unwrap();

        assert!(template.contains(r#"project_name = "myproject""#));
        assert!(!template.contains("{project_name}"));
    }

    #[test]
    fn test_validate_template_name_rejects_path_traversal() {
        assert!(validate_template_name("..").is_err());
        assert!(validate_template_name("../etc").is_err());
        assert!(validate_template_name("foo/../bar").is_err());
    }

    #[test]
    fn test_validate_template_name_rejects_path_separators() {
        assert!(validate_template_name("foo/bar").is_err());
        assert!(validate_template_name("foo\\bar").is_err());
    }

    #[test]
    fn test_validate_template_name_accepts_valid() {
        assert!(validate_template_name("myproject").is_ok());
        assert!(validate_template_name("my-project_2").is_ok());
        assert!(validate_template_name("my.project").is_ok());
    }

    #[test]
    fn test_template_loads_from_file() {
        // Verify that include_str! worked and we have content
        assert!(!TEMPLATE_BASE.is_empty());
        assert!(TEMPLATE_BASE.contains("#let is_final = false"));
        // Settings are now read from YAML config, not hardcoded #let declarations
        assert!(TEMPLATE_BASE.contains("data.config.preview.at(\"show_borders\""));
        assert!(TEMPLATE_BASE.contains("appendix_cfg.at(\"active\""));
        assert!(TEMPLATE_BASE.contains("appendix_cfg.at(\"ref_mode\""));
    }

    /// Renders the real template with `bleed_inside_mm: 0` and checks that page
    /// size, PDF boxes and the inside edge (Typst binding) agree per page.
    fn render_facing(has_cover: bool, first_page_side: crate::models::PageSide) {
        use crate::models::{LayoutPage, PageSide, ProjectState, write_state_yaml};
        use crate::output::typst::{
            BleedSpec, TypstWorld, compile_preview, render_pages_with_world,
        };
        use lopdf::Document;

        let temp = tempfile::TempDir::new().unwrap();
        let mut state = ProjectState::default();
        let book = &mut state.config.book;
        book.page_width_mm = 290.0;
        book.page_height_mm = 290.0;
        book.bleed_mm = 3.0;
        book.bleed_inside_mm = Some(0.0);
        book.cover.active = has_cover;
        book.first_page_side = first_page_side;
        let empty = LayoutPage {
            photos: vec![],
            slots: vec![],
            mode: Default::default(),
        };
        state.layout = vec![empty; if has_cover { 4 } else { 3 }];
        write_state_yaml(&state, &temp.path().join("book.yaml")).unwrap();
        std::fs::write(
            temp.path().join("book.typ"),
            generate_template("book").unwrap(),
        )
        .unwrap();

        let spec = BleedSpec::from_book(&state.config.book);
        let pdf = compile_preview(temp.path(), "book", &spec).unwrap();
        let doc = Document::load(&pdf).unwrap();
        let pt = |v: &lopdf::Object| v.as_float().unwrap() / (72.0 / 25.4);
        let first_inner = usize::from(has_cover);
        let ids: Vec<_> = doc.get_pages().values().copied().collect();
        for (i, id) in ids.iter().enumerate().skip(first_inner) {
            let page = doc.get_object(*id).unwrap().as_dict().unwrap();
            let media: Vec<f32> = page
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(pt)
                .collect();
            let trim: Vec<f32> = page
                .get(b"TrimBox")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(pt)
                .collect();
            assert!(
                (media[2] - 293.0).abs() < 0.01 && (media[3] - 296.0).abs() < 0.01,
                "page {i}: {media:?}"
            );
            assert!(
                (trim[2] - trim[0] - 290.0).abs() < 0.01,
                "page {i}: {trim:?}"
            );
            let expected_x0 = match state.page_side(i).unwrap() {
                PageSide::Right => 0.0,
                PageSide::Left => 3.0,
            };
            assert!((trim[0] - expected_x0).abs() < 0.01, "page {i}: {trim:?}");
        }

        // Red bleed overlay must sit on the outside edge only: Typst's
        // inside/outside binding has to agree with PageSide.
        let mut world = TypstWorld::new(temp.path(), "book").unwrap();
        let idx: Vec<usize> = (first_inner..ids.len()).collect();
        for rendered in render_pages_with_world(&mut world, &idx, 1.0).unwrap() {
            let px = |x: u32| {
                let o = ((rendered.height / 2 * rendered.width + x) * 4) as usize;
                &rendered.pixels[o..o + 3]
            };
            let is_red = |p: &[u8]| p[0] > 200 && p[1] < 80 && p[2] < 80;
            let (left_red, right_red) = (is_red(px(1)), is_red(px(rendered.width - 2)));
            match state.page_side(rendered.page).unwrap() {
                PageSide::Right => assert!(!left_red && right_red, "page {}", rendered.page),
                PageSide::Left => assert!(left_red && !right_red, "page {}", rendered.page),
            }
        }
    }

    #[test]
    fn template_renders_inside_bleed_without_cover() {
        render_facing(false, crate::models::PageSide::Right);
    }

    #[test]
    fn template_renders_inside_bleed_with_cover() {
        render_facing(true, crate::models::PageSide::Right);
    }

    #[test]
    fn template_renders_inside_bleed_first_page_left_without_cover() {
        render_facing(false, crate::models::PageSide::Left);
    }

    #[test]
    fn template_renders_inside_bleed_first_page_left_with_cover() {
        render_facing(true, crate::models::PageSide::Left);
    }

    /// A project YAML written before `bleed_inside_mm` existed (no key at all)
    /// must render exactly as before: 296 mm wide, symmetric TrimBox.
    #[test]
    fn template_renders_symmetric_bleed_without_bleed_inside_key() {
        use crate::models::{LayoutPage, ProjectState, write_state_yaml};
        use crate::output::typst::{BleedSpec, compile_preview};
        use lopdf::Document;

        let temp = tempfile::TempDir::new().unwrap();
        let mut state = ProjectState::default();
        state.config.book.page_width_mm = 290.0;
        state.config.book.page_height_mm = 290.0;
        state.config.book.bleed_mm = 3.0;
        state.layout = vec![
            LayoutPage {
                photos: vec![],
                slots: vec![],
                mode: Default::default(),
            };
            2
        ];
        let yaml_path = temp.path().join("book.yaml");
        write_state_yaml(&state, &yaml_path).unwrap();
        let yaml = std::fs::read_to_string(&yaml_path).unwrap();
        let old_yaml: String = yaml
            .lines()
            .filter(|l| !l.contains("bleed_inside_mm"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_ne!(
            yaml, old_yaml,
            "key must have been present before stripping"
        );
        std::fs::write(&yaml_path, old_yaml).unwrap();
        std::fs::write(
            temp.path().join("book.typ"),
            generate_template("book").unwrap(),
        )
        .unwrap();

        let spec = BleedSpec::from_book(&state.config.book);
        let pdf = compile_preview(temp.path(), "book", &spec).unwrap();
        let doc = Document::load(&pdf).unwrap();
        let mm = |v: &lopdf::Object| v.as_float().unwrap() / (72.0 / 25.4);
        for id in doc.get_pages().values() {
            let page = doc.get_object(*id).unwrap().as_dict().unwrap();
            let media: Vec<f32> = page
                .get(b"MediaBox")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(mm)
                .collect();
            let trim: Vec<f32> = page
                .get(b"TrimBox")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(mm)
                .collect();
            assert!((media[2] - 296.0).abs() < 0.01, "{media:?}");
            assert!(
                (trim[0] - 3.0).abs() < 0.01 && (trim[2] - 293.0).abs() < 0.01,
                "{trim:?}"
            );
        }
    }
}
