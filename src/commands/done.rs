use std::path::Path;

use chrono::Local;

use crate::cli::{OutputFormat, Rating};
use crate::config::Config;
use crate::error::SproutError;
use crate::frontmatter::write_back;
use crate::links;
use crate::note;
use crate::output;
use crate::srs;

pub fn run(
    file: &Path,
    rating: &Rating,
    vault: &Path,
    config: &Config,
    format: &OutputFormat,
) -> Result<(), SproutError> {
    if !file.exists() {
        return Err(SproutError::FileNotFound(file.display().to_string()));
    }

    note::ensure_in_vault(file, vault)?;

    let parsed = note::read_note(file)?;

    // Validate required fields
    let maturity = parsed
        .sprout
        .maturity
        .as_ref()
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?;
    let ease = parsed
        .sprout
        .ease
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?;
    let interval = parsed
        .sprout
        .review_interval
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?;
    let next_review = parsed
        .sprout
        .next_review
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?;

    let raw_yaml = parsed
        .frontmatter_raw
        .as_ref()
        .ok_or_else(|| SproutError::NoFrontmatter(file.display().to_string()))?;

    let today = Local::now().date_naive();
    let link_count = links::count_links(&parsed.body);

    let srs_output = srs::calculate(&srs::SrsInput {
        interval,
        ease,
        next_review,
        today,
        rating: rating.clone(),
        link_count,
        link_weight: config.link_weight(),
        max_interval: config.max_interval(),
    });

    // Determine final next_review with optional load balancing
    let final_next_review = if config.load_balance() {
        let vault_canonical = std::fs::canonicalize(vault)
            .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;
        let exclude_dirs = config.exclude_dirs();
        let all_notes = note::scan_vault_metadata(&vault_canonical, &exclude_dirs)
            .map_err(|e| SproutError::VaultNotFound(e.to_string()))?;

        let existing_dates: Vec<_> = all_notes
            .iter()
            .filter_map(|n| n.sprout.next_review)
            .collect();

        srs::load_balance(srs_output.new_interval, today, &existing_dates)
    } else {
        srs_output.next_review
    };

    // Write back updated frontmatter
    let ease_str = format!("{:.2}", srs_output.new_ease);
    let interval_str = srs_output.new_interval.to_string();
    let next_review_str = final_next_review.to_string();
    let today_str = today.to_string();

    let updates: Vec<(&str, &str)> = vec![
        ("last_review", &today_str),
        ("review_interval", &interval_str),
        ("next_review", &next_review_str),
        ("ease", &ease_str),
    ];

    let content = write_back(raw_yaml, &parsed.body, &updates);
    note::write_note(file, &content)?;

    let file_canonical = std::fs::canonicalize(file)
        .map_err(|_| SproutError::FileNotFound(file.display().to_string()))?;

    output::format_done(
        &file_canonical.to_string_lossy(),
        maturity,
        today,
        srs_output.new_interval,
        final_next_review,
        srs_output.new_ease,
        format,
    );

    Ok(())
}
