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
    let notes = note::scan_vault(vault, exclude_dirs)
        .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

    let today = Local::now().date_naive();

    let mut due: Vec<_> = notes
        .into_iter()
        .filter(|n| {
            // Must be tracked (maturity exists)
            if n.parsed.sprout.maturity.is_none() {
                return false;
            }
            // Skip if next_review is missing
            let next_review = match n.parsed.sprout.next_review {
                Some(nr) => nr,
                None => return false,
            };
            // Skip if ease or review_interval is missing
            if n.parsed.sprout.ease.is_none() || n.parsed.sprout.review_interval.is_none() {
                return false;
            }
            // Due: next_review <= today
            next_review <= today
        })
        .collect();

    // Sort by next_review ascending (most overdue first)
    due.sort_by(|a, b| {
        a.parsed
            .sprout
            .next_review
            .cmp(&b.parsed.sprout.next_review)
    });

    let entries: Vec<_> = due
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
