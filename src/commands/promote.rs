use std::path::Path;

use crate::cli::{Maturity, OutputFormat};
use crate::error::SproutError;
use crate::frontmatter::write_back;
use crate::note;
use crate::output;

pub fn run(
    file: &Path,
    maturity: &Maturity,
    vault: &Path,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    if !file.exists() {
        return Err(SproutError::FileNotFound(file.display().to_string()));
    }

    note::ensure_in_vault(file, vault)?;

    let file_canonical = std::fs::canonicalize(file)
        .map_err(|_| SproutError::FileNotFound(file.display().to_string()))?;
    let vault_canonical = std::fs::canonicalize(vault)
        .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;
    let relative_path = file_canonical
        .strip_prefix(&vault_canonical)
        .unwrap_or(&file_canonical)
        .to_string_lossy()
        .to_string();

    let parsed = note::read_note(file)?;

    let previous_maturity = parsed
        .sprout
        .maturity
        .as_ref()
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?
        .clone();

    let raw_yaml = parsed
        .frontmatter_raw
        .as_ref()
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?;

    let new_maturity = maturity.to_string();

    // Write back (even if same maturity — no-op success with idempotent write)
    let content = write_back(raw_yaml, &parsed.body, &[("maturity", &new_maturity)]);
    note::write_note(file, &content)?;

    output::format_promote(
        &file_canonical.to_string_lossy(),
        &relative_path,
        &previous_maturity,
        &new_maturity,
        parsed.sprout.review_interval,
        parsed.sprout.next_review,
        parsed.sprout.ease,
        format,
    );

    Ok(())
}
