use std::path::Path;

use chrono::Local;

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::error::SproutError;
use crate::frontmatter::{append_field, build_new_frontmatter, has_field, write_back};
use crate::note;
use crate::output;

const SPROUT_FIELDS: &[&str] = &[
    "maturity",
    "created",
    "last_review",
    "review_interval",
    "next_review",
    "ease",
];

pub fn run(
    file: &Path,
    vault: &Path,
    config: &Config,
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

    let today = Local::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let default_ease = config.default_ease();

    let today_str = today.to_string();
    let tomorrow_str = tomorrow.to_string();
    let ease_str = format!("{:.2}", default_ease);

    match &parsed.frontmatter_raw {
        None => {
            // Case A: no frontmatter at all
            let fields: Vec<(&str, &str)> = vec![
                ("maturity", "seedling"),
                ("created", &today_str),
                ("last_review", &today_str),
                ("review_interval", "1"),
                ("next_review", &tomorrow_str),
                ("ease", &ease_str),
            ];
            let content = build_new_frontmatter(&fields, &parsed.body);
            note::write_note(file, &content)?;

            output::format_init(
                &file_canonical.to_string_lossy(),
                &relative_path,
                "seedling",
                1,
                tomorrow,
                default_ease,
                today,
                None,
                format,
            );
        }
        Some(raw_yaml) => {
            // Check which fields exist
            let mut missing: Vec<&str> = Vec::new();
            for &field in SPROUT_FIELDS {
                if !has_field(raw_yaml, field) {
                    missing.push(field);
                }
            }

            if missing.is_empty() {
                // Case D: all fields present
                return Err(SproutError::AlreadyInitialized(
                    file.display().to_string(),
                ));
            }

            let all_missing = missing.len() == SPROUT_FIELDS.len();

            // Build defaults for missing fields
            let defaults: Vec<(&str, String)> = missing
                .iter()
                .map(|&field| {
                    let value = match field {
                        "maturity" => "seedling".to_string(),
                        "created" => today_str.clone(),
                        "last_review" => today_str.clone(),
                        "review_interval" => "1".to_string(),
                        "next_review" => tomorrow_str.clone(),
                        "ease" => ease_str.clone(),
                        _ => unreachable!(),
                    };
                    (field, value)
                })
                .collect();

            // Append missing fields
            let mut yaml = raw_yaml.clone();
            for &(field, ref value) in &defaults {
                yaml = append_field(&yaml, field, value);
            }

            let content = write_back(&yaml, &parsed.body, &[]);
            note::write_note(file, &content)?;

            // Case C: partial — warn about added fields
            if !all_missing {
                let field_names: Vec<String> = missing.iter().map(|s| s.to_string()).collect();
                eprintln!(
                    "warning: missing fields added with defaults: {}",
                    field_names.join(", ")
                );

                // Read back final values from defaults + existing
                let final_maturity = if has_field(raw_yaml, "maturity") {
                    parsed.sprout.maturity.as_deref().unwrap_or("seedling").to_string()
                } else {
                    "seedling".to_string()
                };
                let final_interval = if has_field(raw_yaml, "review_interval") {
                    parsed.sprout.review_interval.unwrap_or(1)
                } else {
                    1
                };
                let final_next_review = if has_field(raw_yaml, "next_review") {
                    parsed.sprout.next_review.unwrap_or(tomorrow)
                } else {
                    tomorrow
                };
                let final_ease = if has_field(raw_yaml, "ease") {
                    parsed.sprout.ease.unwrap_or(default_ease)
                } else {
                    default_ease
                };
                let final_created = if has_field(raw_yaml, "created") {
                    parsed.sprout.created.unwrap_or(today)
                } else {
                    today
                };

                output::format_init(
                    &file_canonical.to_string_lossy(),
                    &relative_path,
                    &final_maturity,
                    final_interval,
                    final_next_review,
                    final_ease,
                    final_created,
                    Some(&field_names),
                    format,
                );
            } else {
                // Case B: frontmatter exists but no sprout fields
                output::format_init(
                    &file_canonical.to_string_lossy(),
                    &relative_path,
                    "seedling",
                    1,
                    tomorrow,
                    default_ease,
                    today,
                    None,
                    format,
                );
            }
        }
    }

    Ok(())
}
