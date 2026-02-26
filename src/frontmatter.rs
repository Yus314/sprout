use chrono::NaiveDate;
use gray_matter::{engine::YAML, Matter};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SproutFrontmatter {
    pub maturity: Option<String>,
    pub created: Option<NaiveDate>,
    pub last_review: Option<NaiveDate>,
    pub review_interval: Option<u32>,
    pub next_review: Option<NaiveDate>,
    pub ease: Option<f64>,
}

pub struct ParsedNote {
    /// Raw YAML string from gray_matter (for string-based write-back)
    pub frontmatter_raw: Option<String>,
    /// Deserialized sprout fields
    pub sprout: SproutFrontmatter,
    /// Note body (content after frontmatter)
    pub body: String,
}

/// Parse note content into frontmatter + body.
/// Normalizes \r\n to \n before parsing.
pub fn parse_note(content: &str) -> ParsedNote {
    let content = content.replace("\r\n", "\n");
    let matter: Matter<YAML> = Matter::new();

    match matter.parse::<SproutFrontmatter>(&content) {
        Ok(parsed) => {
            let raw = if parsed.matter.is_empty() {
                None
            } else {
                Some(parsed.matter)
            };
            ParsedNote {
                frontmatter_raw: raw,
                sprout: parsed.data.unwrap_or_default(),
                body: parsed.content,
            }
        }
        Err(_) => {
            // If parsing fails, treat as no frontmatter
            ParsedNote {
                frontmatter_raw: None,
                sprout: SproutFrontmatter::default(),
                body: content,
            }
        }
    }
}

/// Replace the value of an existing YAML key, preserving inline comments.
/// Pattern: ^(key\s*:\s*)(\S+)(.*)$
pub fn replace_field(yaml: &str, key: &str, new_value: &str) -> String {
    let pattern = format!(r"(?m)^({}\s*:\s*)(\S+)(.*)$", regex::escape(key));
    let re = Regex::new(&pattern).unwrap();
    re.replace(yaml, format!("${{1}}{new_value}${{3}}")).to_string()
}

/// Check if a YAML key exists in the raw YAML string.
pub fn has_field(yaml: &str, key: &str) -> bool {
    let pattern = format!(r"(?m)^{}\s*:", regex::escape(key));
    let re = Regex::new(&pattern).unwrap();
    re.is_match(yaml)
}

/// Append a key: value line at the end of the YAML block.
pub fn append_field(yaml: &str, key: &str, value: &str) -> String {
    let trimmed = yaml.trim_end_matches('\n');
    if trimmed.is_empty() {
        format!("{key}: {value}\n")
    } else {
        format!("{trimmed}\n{key}: {value}\n")
    }
}

/// Update multiple fields in the raw YAML: replace if exists, append if not.
/// Returns the reconstructed full file content with `---` delimiters.
pub fn write_back(raw_yaml: &str, body: &str, updates: &[(&str, &str)]) -> String {
    let mut yaml = raw_yaml.to_string();
    for &(key, value) in updates {
        if has_field(&yaml, key) {
            yaml = replace_field(&yaml, key, value);
        } else {
            yaml = append_field(&yaml, key, value);
        }
    }
    // Ensure yaml ends with newline
    let yaml = yaml.trim_end_matches('\n');
    format!("---\n{yaml}\n---\n{body}")
}

/// Build new frontmatter block for a file that has none.
pub fn build_new_frontmatter(fields: &[(&str, &str)], body: &str) -> String {
    let mut yaml_lines = Vec::new();
    for &(key, value) in fields {
        yaml_lines.push(format!("{key}: {value}"));
    }
    let yaml = yaml_lines.join("\n");
    format!("---\n{yaml}\n---\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_with_frontmatter() {
        let content = "---\nmaturity: seedling\nease: 2.5\n---\nHello world\n";
        let parsed = parse_note(content);
        assert!(parsed.frontmatter_raw.is_some());
        assert_eq!(parsed.sprout.maturity.as_deref(), Some("seedling"));
        assert_eq!(parsed.sprout.ease, Some(2.5));
        assert_eq!(parsed.body.trim(), "Hello world");
    }

    #[test]
    fn test_parse_note_without_frontmatter() {
        let content = "Hello world\n";
        let parsed = parse_note(content);
        assert!(parsed.frontmatter_raw.is_none());
        assert!(parsed.sprout.maturity.is_none());
    }

    #[test]
    fn test_parse_note_crlf_normalization() {
        let content = "---\r\nmaturity: seedling\r\n---\r\nHello\r\n";
        let parsed = parse_note(content);
        assert!(parsed.frontmatter_raw.is_some());
        assert_eq!(parsed.sprout.maturity.as_deref(), Some("seedling"));
    }

    #[test]
    fn test_replace_field_preserves_comment() {
        let yaml = "review_interval: 3  # days\nease: 2.50\n";
        let result = replace_field(yaml, "review_interval", "7");
        assert!(result.contains("review_interval: 7  # days"));
        assert!(result.contains("ease: 2.50"));
    }

    #[test]
    fn test_append_field() {
        let yaml = "maturity: seedling\n";
        let result = append_field(yaml, "ease", "2.50");
        assert!(result.contains("maturity: seedling\n"));
        assert!(result.contains("ease: 2.50\n"));
    }

    #[test]
    fn test_has_field() {
        let yaml = "maturity: seedling\nease: 2.50\n";
        assert!(has_field(yaml, "maturity"));
        assert!(has_field(yaml, "ease"));
        assert!(!has_field(yaml, "created"));
    }

    #[test]
    fn test_write_back_roundtrip() {
        let yaml = "tags: [rust]\nmaturity: seedling\nease: 2.50\n";
        let body = "Some content\n";
        let result = write_back(yaml, body, &[("ease", "2.65"), ("next_review", "2026-03-01")]);
        assert!(result.contains("tags: [rust]"));
        assert!(result.contains("ease: 2.65"));
        assert!(result.contains("next_review: 2026-03-01"));
        assert!(result.contains("maturity: seedling"));
        assert!(result.starts_with("---\n"));
        assert!(result.contains("\n---\nSome content\n"));
    }

    #[test]
    fn test_build_new_frontmatter() {
        let body = "Hello world\n";
        let result = build_new_frontmatter(
            &[("maturity", "seedling"), ("ease", "2.50")],
            body,
        );
        assert_eq!(result, "---\nmaturity: seedling\nease: 2.50\n---\nHello world\n");
    }

    #[test]
    fn test_parse_note_with_unknown_keys() {
        let content = "---\ntags: [rust, zettelkasten]\nmaturity: budding\ncssclasses: note\n---\nBody\n";
        let parsed = parse_note(content);
        assert_eq!(parsed.sprout.maturity.as_deref(), Some("budding"));
        // Unknown keys should not cause errors
        assert!(parsed.frontmatter_raw.is_some());
        let raw = parsed.frontmatter_raw.unwrap();
        assert!(raw.contains("tags:"));
        assert!(raw.contains("cssclasses:"));
    }
}
