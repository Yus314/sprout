use std::path::Path;

use chrono::Local;

use crate::cli::OutputFormat;
use crate::error::SproutError;
use crate::links;
use crate::note;
use crate::output;

pub fn run(
    file: &Path,
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
    let path_str = file_canonical.to_string_lossy().to_string();

    // Tracked = maturity field exists
    match &parsed.sprout.maturity {
        Some(maturity) => {
            let today = Local::now().date_naive();
            let link_count = links::count_links(&parsed.body);

            let is_due = parsed
                .sprout
                .next_review
                .map(|nr| nr <= today)
                .unwrap_or(false);

            let days_until_review = parsed
                .sprout
                .next_review
                .map(|nr| (nr - today).num_days())
                .unwrap_or(0);

            output::format_show_tracked(
                &path_str,
                &relative_path,
                maturity,
                parsed.sprout.created,
                parsed.sprout.last_review,
                parsed.sprout.review_interval,
                parsed.sprout.next_review,
                parsed.sprout.ease,
                is_due,
                days_until_review,
                link_count,
                format,
            );
        }
        None => {
            output::format_show_untracked(&path_str, &relative_path, format);
        }
    }

    Ok(())
}
