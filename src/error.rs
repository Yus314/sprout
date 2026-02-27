use crate::cli::OutputFormat;

#[derive(Debug, thiserror::Error)]
pub enum SproutError {
    #[error("{0}: file not found")]
    FileNotFound(String),

    #[error("{0}: file is outside vault")]
    OutsideVault(String),

    #[error("{0}: missing required sprout frontmatter fields")]
    NoFrontmatter(String),

    #[error("vault not found: {0}")]
    VaultNotFound(String),

    #[error("{0}: already initialized with all sprout fields")]
    AlreadyInitialized(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("{0}: invalid note title")]
    InvalidTitle(String),
}

impl SproutError {
    pub fn error_code(&self) -> &str {
        match self {
            SproutError::FileNotFound(_) => "file_not_found",
            SproutError::OutsideVault(_) => "outside_vault",
            SproutError::NoFrontmatter(_) => "no_frontmatter",
            SproutError::VaultNotFound(_) => "vault_not_found",
            SproutError::AlreadyInitialized(_) => "already_initialized",
            SproutError::ParseError(_) => "parse_error",
            SproutError::InvalidTitle(_) => "invalid_title",
        }
    }
}

pub fn format_error(error: &SproutError, format: &OutputFormat) {
    match format {
        OutputFormat::Human => {
            eprintln!("error: {error}");
        }
        OutputFormat::Json => {
            let json = serde_json::json!({
                "error": error.error_code(),
                "message": error.to_string(),
            });
            eprintln!("{}", serde_json::to_string(&json).unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            SproutError::FileNotFound("x".into()).error_code(),
            "file_not_found"
        );
        assert_eq!(
            SproutError::OutsideVault("x".into()).error_code(),
            "outside_vault"
        );
        assert_eq!(
            SproutError::NoFrontmatter("x".into()).error_code(),
            "no_frontmatter"
        );
        assert_eq!(
            SproutError::VaultNotFound("x".into()).error_code(),
            "vault_not_found"
        );
        assert_eq!(
            SproutError::AlreadyInitialized("x".into()).error_code(),
            "already_initialized"
        );
        assert_eq!(
            SproutError::ParseError("x".into()).error_code(),
            "parse_error"
        );
        assert_eq!(
            SproutError::InvalidTitle("x".into()).error_code(),
            "invalid_title"
        );
    }

    #[test]
    fn test_error_display() {
        let e = SproutError::FileNotFound("/foo.md".into());
        assert_eq!(e.to_string(), "/foo.md: file not found");

        let e = SproutError::OutsideVault("/outside.md".into());
        assert_eq!(e.to_string(), "/outside.md: file is outside vault");

        let e = SproutError::NoFrontmatter("note.md".into());
        assert_eq!(
            e.to_string(),
            "note.md: missing required sprout frontmatter fields"
        );

        let e = SproutError::AlreadyInitialized("note.md".into());
        assert_eq!(
            e.to_string(),
            "note.md: already initialized with all sprout fields"
        );
    }
}
