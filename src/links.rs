use regex::Regex;
use std::collections::HashSet;

/// Count unique internal links in the note body.
/// Supports [[wiki-link]] and [text](path) formats.
/// Excludes external URLs (http:// or https://) and image links (![...](path)).
pub fn count_links(body: &str) -> usize {
    let mut targets = HashSet::new();

    // [[wiki-link]] — extract target before optional |display text
    let wiki_re = Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").unwrap();
    for cap in wiki_re.captures_iter(body) {
        let target = cap[1].trim();
        if !target.starts_with("http://") && !target.starts_with("https://") {
            targets.insert(target.to_string());
        }
    }

    // [text](path) — exclude images (preceded by !) and external URLs
    let md_re = Regex::new(r"(?:^|[^!])\[([^\]]*)\]\(([^)]+)\)").unwrap();
    for cap in md_re.captures_iter(body) {
        let path = cap[2].trim();
        if !path.starts_with("http://") && !path.starts_with("https://") {
            targets.insert(path.to_string());
        }
    }

    targets.len()
}

/// Calculate link factor: normalized 0.0-1.0 value based on link count.
/// Formula: max(0.0, min(1.0, ln(link_count + 0.5) / ln(64)))
pub fn link_factor(link_count: usize) -> f64 {
    let value = ((link_count as f64) + 0.5).ln() / 64.0_f64.ln();
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiki_links() {
        let body = "See [[note1]] and [[note2|Display Name]].";
        assert_eq!(count_links(body), 2);
    }

    #[test]
    fn test_markdown_links() {
        let body = "See [link](path/to/note.md) and [other](another.md).";
        assert_eq!(count_links(body), 2);
    }

    #[test]
    fn test_deduplication() {
        let body = "See [[note1]] and [[note1]] again, plus [[note1|alias]].";
        assert_eq!(count_links(body), 1);
    }

    #[test]
    fn test_exclude_external_urls() {
        let body = "See [google](https://google.com) and [[https://example.com]].";
        assert_eq!(count_links(body), 0);
    }

    #[test]
    fn test_exclude_images() {
        let body = "![alt](image.png) but [link](note.md)";
        assert_eq!(count_links(body), 1);
    }

    #[test]
    fn test_mixed_links() {
        let body = "[[wiki]] and [md](path.md) and ![img](pic.png) and [ext](https://x.com)";
        assert_eq!(count_links(body), 2);
    }

    #[test]
    fn test_link_factor_zero_links() {
        let f = link_factor(0);
        assert!(f == 0.0, "0 links should give factor 0.0, got {f}");
    }

    #[test]
    fn test_link_factor_eight_links() {
        let f = link_factor(8);
        assert!((f - 0.5).abs() < 0.05, "8 links should give factor ≈0.5, got {f}");
    }

    #[test]
    fn test_link_factor_64_links() {
        let f = link_factor(64);
        assert!((f - 1.0).abs() < 0.01, "64 links should give factor ≈1.0, got {f}");
    }

    #[test]
    fn test_link_factor_clamped_to_one() {
        let f = link_factor(1000);
        assert!(f <= 1.0);
    }
}
