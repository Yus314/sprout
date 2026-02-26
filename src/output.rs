use chrono::NaiveDate;
use colored::Colorize;
use serde_json::{json, Value};
use crate::cli::OutputFormat;

// ── review / list ──────────────────────────────────────────────────

pub fn format_note_list(
    notes: &[(String, String, Option<String>, Option<u32>, Option<NaiveDate>, Option<f64>)],
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let arr: Vec<Value> = notes
                .iter()
                .map(|(path, rel, maturity, interval, next_review, ease)| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("path".into(), json!(path));
                    obj.insert("relative_path".into(), json!(rel));
                    obj.insert("maturity".into(), json!(maturity));
                    obj.insert(
                        "review_interval".into(),
                        match interval {
                            Some(v) => json!(v),
                            None => Value::Null,
                        },
                    );
                    obj.insert(
                        "next_review".into(),
                        match next_review {
                            Some(d) => json!(d.to_string()),
                            None => Value::Null,
                        },
                    );
                    obj.insert(
                        "ease".into(),
                        match ease {
                            Some(v) => json!(v),
                            None => Value::Null,
                        },
                    );
                    Value::Object(obj)
                })
                .collect();
            println!("{}", serde_json::to_string(&arr).unwrap());
        }
        OutputFormat::Human => {
            if notes.is_empty() {
                println!("No notes found.");
                return;
            }
            for (_, rel, maturity, interval, next_review, _) in notes {
                let mat = maturity.as_deref().unwrap_or("unknown");
                let int_str = interval.map(|i| format!("{i}d")).unwrap_or_else(|| "-".into());
                let nr_str = next_review
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "-".into());
                let colored_mat = match mat {
                    "seedling" => mat.green(),
                    "budding" => mat.yellow(),
                    "evergreen" => mat.cyan(),
                    _ => mat.normal(),
                };
                println!("  {rel}  [{colored_mat}]  interval: {int_str}  next: {nr_str}");
            }
        }
    }
}

// ── done ───────────────────────────────────────────────────────────

pub fn format_done(
    path: &str,
    maturity: &str,
    last_review: NaiveDate,
    new_interval: u32,
    next_review: NaiveDate,
    ease: f64,
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let obj = json!({
                "path": path,
                "maturity": maturity,
                "last_review": last_review.to_string(),
                "new_interval": new_interval,
                "next_review": next_review.to_string(),
                "ease": ease,
            });
            println!("{}", serde_json::to_string(&obj).unwrap());
        }
        OutputFormat::Human => {
            println!(
                "Reviewed: {} → interval {}d, next: {}",
                maturity, new_interval, next_review
            );
        }
    }
}

// ── stats ──────────────────────────────────────────────────────────

pub fn format_stats(
    total: usize,
    seedling: usize,
    budding: usize,
    evergreen: usize,
    due_today: usize,
    overdue: usize,
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let obj = json!({
                "total": total,
                "seedling": seedling,
                "budding": budding,
                "evergreen": evergreen,
                "due_today": due_today,
                "overdue": overdue,
            });
            println!("{}", serde_json::to_string(&obj).unwrap());
        }
        OutputFormat::Human => {
            println!("Total: {total} (seedling: {seedling}, budding: {budding}, evergreen: {evergreen})");
            println!("Due today: {due_today}, Overdue: {overdue}");
        }
    }
}

// ── promote ────────────────────────────────────────────────────────

pub fn format_promote(
    path: &str,
    relative_path: &str,
    previous_maturity: &str,
    new_maturity: &str,
    review_interval: Option<u32>,
    next_review: Option<NaiveDate>,
    ease: Option<f64>,
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let obj = json!({
                "path": path,
                "relative_path": relative_path,
                "previous_maturity": previous_maturity,
                "new_maturity": new_maturity,
                "review_interval": review_interval,
                "next_review": next_review.map(|d| d.to_string()),
                "ease": ease,
            });
            println!("{}", serde_json::to_string(&obj).unwrap());
        }
        OutputFormat::Human => {
            println!("Promoted: {previous_maturity} → {new_maturity}");
        }
    }
}

// ── init ───────────────────────────────────────────────────────────

pub fn format_init(
    path: &str,
    relative_path: &str,
    maturity: &str,
    review_interval: u32,
    next_review: NaiveDate,
    ease: f64,
    created: NaiveDate,
    fields_added: Option<&[String]>,
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let mut obj = serde_json::Map::new();
            obj.insert("path".into(), json!(path));
            obj.insert("relative_path".into(), json!(relative_path));
            obj.insert("maturity".into(), json!(maturity));
            obj.insert("review_interval".into(), json!(review_interval));
            obj.insert("next_review".into(), json!(next_review.to_string()));
            obj.insert("ease".into(), json!(ease));
            obj.insert("created".into(), json!(created.to_string()));
            if let Some(fields) = fields_added {
                obj.insert("fields_added".into(), json!(fields));
            }
            println!("{}", serde_json::to_string(&Value::Object(obj)).unwrap());
        }
        OutputFormat::Human => {
            println!("Initialized: {relative_path} [{maturity}]");
            if let Some(fields) = fields_added {
                println!("  fields added: {}", fields.join(", "));
            }
        }
    }
}

// ── show ───────────────────────────────────────────────────────────

pub fn format_show_tracked(
    path: &str,
    relative_path: &str,
    maturity: &str,
    created: Option<NaiveDate>,
    last_review: Option<NaiveDate>,
    review_interval: Option<u32>,
    next_review: Option<NaiveDate>,
    ease: Option<f64>,
    is_due: bool,
    days_until_review: i64,
    link_count: usize,
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let obj = json!({
                "path": path,
                "relative_path": relative_path,
                "tracked": true,
                "maturity": maturity,
                "created": created.map(|d| d.to_string()),
                "last_review": last_review.map(|d| d.to_string()),
                "review_interval": review_interval,
                "next_review": next_review.map(|d| d.to_string()),
                "ease": ease,
                "is_due": is_due,
                "days_until_review": days_until_review,
                "link_count": link_count,
            });
            println!("{}", serde_json::to_string(&obj).unwrap());
        }
        OutputFormat::Human => {
            let mat_colored = match maturity {
                "seedling" => maturity.green(),
                "budding" => maturity.yellow(),
                "evergreen" => maturity.cyan(),
                _ => maturity.normal(),
            };
            println!("{relative_path} [{mat_colored}]");
            if let Some(d) = created {
                println!("  Created: {d}");
            }
            if let Some(d) = last_review {
                println!("  Last review: {d}");
            }
            if let Some(i) = review_interval {
                println!("  Interval: {i}d");
            }
            if let Some(d) = next_review {
                println!("  Next review: {d}");
            }
            if let Some(e) = ease {
                println!("  Ease: {e:.2}");
            }
            let due_str = if is_due { "YES".red().to_string() } else { "no".to_string() };
            println!("  Due: {due_str} ({days_until_review}d)");
            println!("  Links: {link_count}");
        }
    }
}

pub fn format_show_untracked(path: &str, relative_path: &str, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            let obj = json!({
                "path": path,
                "relative_path": relative_path,
                "tracked": false,
            });
            println!("{}", serde_json::to_string(&obj).unwrap());
        }
        OutputFormat::Human => {
            println!("{relative_path} [not tracked]");
        }
    }
}

