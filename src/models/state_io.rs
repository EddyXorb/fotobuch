use anyhow::{Context, Result};
use std::path::Path;

use super::ProjectState;

/// Read project state from a YAML file path.
pub fn read_state_yaml(path: &Path) -> Result<ProjectState> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let state: ProjectState = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML from {}", path.display()))?;

    Ok(state)
}

/// Write project state to a YAML file path.
pub fn write_state_yaml(state: &ProjectState, path: &Path) -> Result<()> {
    let yaml = serde_yaml::to_string(state).context("Failed to serialize project state to YAML")?;
    let yaml = annotate_page_indices(&yaml);

    std::fs::write(path, yaml).with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Insert a `# page N` comment (0-based) before each `LayoutPage` in the
/// serialized YAML, so the human-readable index is visible despite the page
/// carrying no `page` field. The comments are read-only annotations and are
/// ignored on parse — a page's identity stays its position in `layout[]`.
///
/// Relies on `layout` being the last field of [`ProjectState`]: once the
/// top-level `layout:` key is seen, every following top-level sequence item
/// (`- ` at column 0) starts a new page.
fn annotate_page_indices(yaml: &str) -> String {
    let mut out = String::with_capacity(yaml.len() + yaml.len() / 16);
    let mut in_layout = false;
    let mut page_idx = 0usize;

    for line in yaml.lines() {
        if !in_layout {
            in_layout = line == "layout:";
        } else if line.starts_with("- ") {
            out.push_str(&format!("# page {page_idx}\n"));
            page_idx += 1;
        }
        out.push_str(line);
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotates_each_page_with_zero_based_index() {
        let yaml = "\
config: {}
photos: []
layout:
- photos:
  - a.jpg
  slots: []
- photos:
  - b.jpg
  slots: []
";
        let out = annotate_page_indices(yaml);

        assert!(out.contains("# page 0\n- photos:\n  - a.jpg"));
        assert!(out.contains("# page 1\n- photos:\n  - b.jpg"));
        assert_eq!(out.matches("# page").count(), 2);
    }

    #[test]
    fn empty_layout_gets_no_comments() {
        let yaml = "config: {}\nphotos: []\nlayout: []\n";
        assert!(!annotate_page_indices(yaml).contains("# page"));
    }

    #[test]
    fn does_not_annotate_sequence_items_before_layout() {
        // `photos` is also a sequence; its items must stay untouched.
        let yaml = "\
config: {}
photos:
- dir: A
  photos: []
layout:
- photos:
  - a.jpg
  slots: []
";
        let out = annotate_page_indices(yaml);
        assert_eq!(out.matches("# page").count(), 1);
        assert!(out.contains("# page 0\n- photos:\n  - a.jpg"));
    }

    #[test]
    fn annotated_yaml_round_trips_back_to_state() {
        use crate::models::LayoutPage;

        let mut state = ProjectState::default();
        state.layout.push(LayoutPage {
            photos: vec!["a.jpg".to_string()],
            slots: vec![],
            mode: Default::default(),
        });

        let yaml = serde_yaml::to_string(&state).unwrap();
        let annotated = annotate_page_indices(&yaml);
        assert!(annotated.contains("# page 0"));

        // Comments must be ignored on parse.
        let parsed: ProjectState = serde_yaml::from_str(&annotated).unwrap();
        assert_eq!(parsed.layout.len(), 1);
        assert_eq!(parsed.layout[0].photos, vec!["a.jpg"]);
    }
}
