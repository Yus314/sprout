use std::path::Path;

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::error::SproutError;
use crate::note;
use crate::output;
use crate::template;

pub fn run_list(
    vault: &Path,
    config: &Config,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    let paths = note::scan_vault_paths(vault, &config.exclude_dirs())
        .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

    let mut candidates: Vec<(String, String)> = paths
        .into_iter()
        .map(|n| (n.path.to_string_lossy().to_string(), n.relative_path))
        .collect();

    candidates.sort_by(|a, b| a.1.cmp(&b.1));

    output::format_note_candidates(&candidates, format);
    Ok(())
}

pub fn run_create(
    title: &str,
    vault: &Path,
    config: &Config,
    template_name: Option<&str>,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    // Validate title
    validate_title(title)?;

    // Strip .md suffix if present
    let title = title.strip_suffix(".md").unwrap_or(title);

    let vault_canonical = std::fs::canonicalize(vault)
        .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;
    let file_path = vault_canonical.join(format!("{title}.md"));
    let relative_path = format!("{title}.md");

    if file_path.exists() {
        // Idempotent: return existing file info
        output::format_note_created(
            &file_path.to_string_lossy(),
            &relative_path,
            false,
            false,
            format,
        );
        return Ok(());
    }

    // Load and expand template
    let tmpl_name = template_name.unwrap_or_else(|| config.default_template());
    let template_content = template::load_template(&config.template_dir(), tmpl_name)?;
    let today = chrono::Local::now().date_naive().to_string();
    let expanded = template::expand(
        &template_content,
        title,
        &today,
        config.allow_template_exec(),
    )?;

    // Write the file
    note::write_note(&file_path, &expanded)?;

    // Auto-init if configured
    let initialized = if config.auto_init() {
        match super::init::init_note(&file_path, vault, config) {
            Ok(_) => true,
            Err(SproutError::AlreadyInitialized(_)) => false,
            Err(e) => return Err(e),
        }
    } else {
        false
    };

    output::format_note_created(
        &file_path.to_string_lossy(),
        &relative_path,
        true,
        initialized,
        format,
    );

    Ok(())
}

fn validate_title(title: &str) -> Result<(), SproutError> {
    if title.is_empty() {
        return Err(SproutError::InvalidTitle("(empty)".into()));
    }
    if title.contains('/') || title.contains('\\') || title.contains('\0') {
        return Err(SproutError::InvalidTitle(title.into()));
    }
    if title.contains("..") {
        return Err(SproutError::InvalidTitle(title.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_title_valid() {
        assert!(validate_title("My Note").is_ok());
        assert!(validate_title("日本語ノート").is_ok());
        assert!(validate_title("note-with-dashes").is_ok());
        assert!(validate_title("note.md").is_ok());
    }

    #[test]
    fn test_validate_title_empty() {
        assert!(validate_title("").is_err());
    }

    #[test]
    fn test_validate_title_slash() {
        assert!(validate_title("sub/note").is_err());
        assert!(validate_title("sub\\note").is_err());
    }

    #[test]
    fn test_validate_title_traversal() {
        assert!(validate_title("../escape").is_err());
        assert!(validate_title("a..b").is_err());
    }

    #[test]
    fn test_validate_title_null_byte() {
        assert!(validate_title("note\0bad").is_err());
    }
}
