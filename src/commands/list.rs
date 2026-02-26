use std::path::Path;

use crate::cli::{Maturity, OutputFormat};
use crate::error::SproutError;
use crate::note;
use crate::output;

pub fn run(
    vault: &Path,
    maturity_filter: Option<&Maturity>,
    exclude_dirs: &[String],
    format: &OutputFormat,
) -> Result<(), SproutError> {
    let notes = note::scan_vault(vault, exclude_dirs)
        .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

    let mut tracked: Vec<_> = notes
        .into_iter()
        .filter(|n| n.parsed.sprout.maturity.is_some())
        .filter(|n| {
            if let Some(filter) = maturity_filter {
                n.parsed.sprout.maturity.as_deref() == Some(&filter.to_string())
            } else {
                true
            }
        })
        .collect();

    // Sort by relative_path alphabetical ascending
    tracked.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let entries: Vec<_> = tracked
        .iter()
        .map(|n| {
            (
                n.path.to_string_lossy().to_string(),
                n.relative_path.clone(),
                n.parsed.sprout.maturity.clone(),
                n.parsed.sprout.review_interval,
                n.parsed.sprout.next_review,
                n.parsed.sprout.ease,
            )
        })
        .collect();

    output::format_note_list(&entries, format);
    Ok(())
}
