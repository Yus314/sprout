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
    fn test_no_links() {
        assert_eq!(count_links("No links here at all."), 0);
    }

    #[test]
    fn test_empty_body() {
        assert_eq!(count_links(""), 0);
    }

    #[test]
    fn test_multiline_links() {
        let body = "First [[note1]]\nSecond [[note2]]\nThird [link](path.md)\n";
        assert_eq!(count_links(body), 3);
    }

    #[test]
    fn test_wiki_link_with_heading() {
        let body = "See [[note#heading]].";
        assert_eq!(count_links(body), 1);
    }

    #[test]
    fn test_mixed_deduplication() {
        // Same target via wiki-link and markdown link should count as separate
        // (wiki target = "note", md target = "note.md")
        let body = "[[note]] and [link](note.md)";
        assert_eq!(count_links(body), 2);
    }

    #[test]
    fn test_http_wiki_link_excluded() {
        let body = "[[http://example.com]]";
        assert_eq!(count_links(body), 0);
    }

    #[test]
    fn test_image_at_start_of_line() {
        let body = "![alt](image.png)";
        assert_eq!(count_links(body), 0);
    }

    #[test]
    fn test_link_factor_one_link() {
        let f = link_factor(1);
        assert!(f > 0.0 && f < 0.5, "1 link factor should be small, got {f}");
    }

    #[test]
    fn test_link_factor_monotonic() {
        // link_factor should be monotonically increasing
        let f1 = link_factor(1);
        let f5 = link_factor(5);
        let f10 = link_factor(10);
        let f50 = link_factor(50);
        assert!(f1 < f5);
        assert!(f5 < f10);
        assert!(f10 < f50);
    }

    #[test]
    fn test_link_factor_clamped_to_one() {
        let f = link_factor(1000);
        assert!(f <= 1.0);
    }
}
