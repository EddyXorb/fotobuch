//! Project name validation

use anyhow::{bail, Result};

/// Validate a project name against the naming rules
///
/// Rules:
/// - Must start with [a-zA-Z]
/// - Can only contain [a-zA-Z0-9._-]
/// - Maximum length: 50 characters
/// - Cannot contain ".." (path traversal)
/// - Cannot be "fotobuch" (reserved as branch prefix)
pub fn validate_project_name(name: &str) -> Result<()> {
    // Check length
    if name.is_empty() {
        bail!("Project name cannot be empty");
    }
    if name.len() > 50 {
        bail!("Project name cannot be longer than 50 characters");
    }

    // Check reserved name
    if name == "fotobuch" {
        bail!("Project name 'fotobuch' is reserved");
    }

    // Check path traversal
    if name.contains("..") {
        bail!("Project name cannot contain '..'");
    }

    // Check first character
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        bail!("Project name must start with a letter (a-z, A-Z)");
    }

    // Check all characters
    for ch in name.chars() {
        if !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-') {
            bail!("Project name can only contain letters, numbers, dots, underscores, and hyphens");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_project_name("myproject").is_ok());
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("my_project").is_ok());
        assert!(validate_project_name("my.project").is_ok());
        assert!(validate_project_name("Project123").is_ok());
        assert!(validate_project_name("a").is_ok());
    }

    #[test]
    fn test_empty_name() {
        assert!(validate_project_name("").is_err());
    }

    #[test]
    fn test_too_long() {
        let long_name = "a".repeat(51);
        assert!(validate_project_name(&long_name).is_err());
    }

    #[test]
    fn test_reserved_name() {
        assert!(validate_project_name("fotobuch").is_err());
    }

    #[test]
    fn test_path_traversal() {
        assert!(validate_project_name("..").is_err());
        assert!(validate_project_name("../etc").is_err());
        assert!(validate_project_name("foo..bar").is_err());
    }

    #[test]
    fn test_invalid_start() {
        assert!(validate_project_name("1project").is_err());
        assert!(validate_project_name("-project").is_err());
        assert!(validate_project_name("_project").is_err());
        assert!(validate_project_name(".project").is_err());
    }

    #[test]
    fn test_invalid_characters() {
        assert!(validate_project_name("my project").is_err()); // space
        assert!(validate_project_name("my/project").is_err()); // slash
        assert!(validate_project_name("my\\project").is_err()); // backslash
        assert!(validate_project_name("my@project").is_err()); // @
        assert!(validate_project_name("my#project").is_err()); // #
    }
}
