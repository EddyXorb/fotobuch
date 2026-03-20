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
        assert!(TEMPLATE_BASE.contains("#let appendix_show = false"));
        assert!(TEMPLATE_BASE.contains("#let appendix_ref_mode = \"positions\""));
    }
}
