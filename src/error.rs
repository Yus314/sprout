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
