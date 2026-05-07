use std::fmt;

#[derive(Debug)]
pub struct ValidationError(pub String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// GCP project ID: 6–30 chars, lowercase letters/digits/hyphens,
/// must start with a letter, must not end with a hyphen.
pub fn validate_project_id(id: &str) -> Result<(), ValidationError> {
    if id.len() < 6 || id.len() > 30 {
        return Err(ValidationError(format!(
            "Project ID must be 6–30 characters (got {})",
            id.len()
        )));
    }
    if !id.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(ValidationError(
            "Project ID must start with a lowercase letter".to_string(),
        ));
    }
    if id.ends_with('-') {
        return Err(ValidationError(
            "Project ID must not end with a hyphen".to_string(),
        ));
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(ValidationError(
            "Project ID may only contain lowercase letters, digits, and hyphens".to_string(),
        ));
    }
    Ok(())
}

/// Cloud SQL instance name: 1–98 chars, lowercase letters/digits/hyphens,
/// must start with a letter.
pub fn validate_instance_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() || name.len() > 98 {
        return Err(ValidationError(format!(
            "Instance name must be 1–98 characters (got {})",
            name.len()
        )));
    }
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(ValidationError(
            "Instance name must start with a lowercase letter".to_string(),
        ));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(ValidationError(
            "Instance name may only contain lowercase letters, digits, and hyphens".to_string(),
        ));
    }
    Ok(())
}

/// Backup name: 1–63 chars, letters/digits/hyphens/underscores.
pub fn validate_backup_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() || name.len() > 63 {
        return Err(ValidationError(format!(
            "Backup name must be 1–63 characters (got {})",
            name.len()
        )));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(ValidationError(
            "Backup name may only contain letters, digits, hyphens, and underscores".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_id_valid() {
        assert!(validate_project_id("my-project-123").is_ok());
        assert!(validate_project_id("abcdef").is_ok());
    }

    #[test]
    fn test_project_id_too_short() {
        assert!(validate_project_id("abc").is_err());
    }

    #[test]
    fn test_project_id_too_long() {
        assert!(validate_project_id(&"a".repeat(31)).is_err());
    }

    #[test]
    fn test_project_id_starts_with_digit() {
        assert!(validate_project_id("1invalid").is_err());
    }

    #[test]
    fn test_project_id_ends_with_hyphen() {
        assert!(validate_project_id("valid-id-").is_err());
    }

    #[test]
    fn test_project_id_uppercase() {
        assert!(validate_project_id("MyProject").is_err());
    }

    #[test]
    fn test_instance_name_valid() {
        assert!(validate_instance_name("my-instance").is_ok());
        assert!(validate_instance_name("a").is_ok());
    }

    #[test]
    fn test_instance_name_starts_with_digit() {
        assert!(validate_instance_name("1bad").is_err());
    }

    #[test]
    fn test_backup_name_valid() {
        assert!(validate_backup_name("daily-backup_01").is_ok());
    }

    #[test]
    fn test_backup_name_too_long() {
        assert!(validate_backup_name(&"a".repeat(64)).is_err());
    }
}
