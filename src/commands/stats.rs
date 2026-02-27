use std::path::Path;

use chrono::Local;

use crate::cli::OutputFormat;
use crate::error::SproutError;
use crate::note;
use crate::output;

pub fn run(
    vault: &Path,
    exclude_dirs: &[String],
    format: &OutputFormat,
) -> Result<(), SproutError> {
    let notes = note::scan_vault_metadata(vault, exclude_dirs)
        .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

    let today = Local::now().date_naive();

    let tracked: Vec<_> = notes
        .iter()
        .filter(|n| n.sprout.maturity.is_some())
        .collect();

    let total = tracked.len();
    let seedling = tracked
        .iter()
        .filter(|n| n.sprout.maturity.as_deref() == Some("seedling"))
        .count();
    let budding = tracked
        .iter()
        .filter(|n| n.sprout.maturity.as_deref() == Some("budding"))
        .count();
    let evergreen = tracked
        .iter()
        .filter(|n| n.sprout.maturity.as_deref() == Some("evergreen"))
        .count();

    let due_today = tracked
        .iter()
        .filter(|n| n.sprout.next_review == Some(today))
        .count();

    let overdue = tracked
        .iter()
        .filter(|n| {
            n.sprout
                .next_review
                .map(|nr| nr < today)
                .unwrap_or(false)
        })
        .count();

    output::format_stats(total, seedling, budding, evergreen, due_today, overdue, format);
    Ok(())
}
