use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn sprout() -> Command {
    Command::cargo_bin("sprout").unwrap()
}

/// Copy a fixture file into a temp vault directory and return (dir, file_path).
fn setup_vault(fixture_name: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let fixture_src = std::path::Path::new("tests/fixtures").join(fixture_name);
    let dest = dir.path().join(fixture_name);
    fs::copy(&fixture_src, &dest).unwrap();
    (dir, dest)
}

/// Setup a vault with multiple files
fn setup_vault_multi(fixtures: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for name in fixtures {
        let src = std::path::Path::new("tests/fixtures").join(name);
        let dest = dir.path().join(name);
        fs::copy(&src, &dest).unwrap();
    }
    dir
}

// ── init ───────────────────────────────────────────────────────────

#[test]
fn init_no_frontmatter_creates_fields() {
    let (dir, file) = setup_vault("no_frontmatter.md");
    sprout()
        .args(["init", file.to_str().unwrap(), "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"maturity\":\"seedling\""))
        .stdout(predicate::str::contains("\"review_interval\":1"))
        .stdout(predicate::str::contains("\"ease\":2.5"));

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("maturity: seedling"));
    assert!(content.contains("ease: 2.50"));
    assert!(content.contains("review_interval: 1"));
}

#[test]
fn init_already_initialized_errors() {
    let (dir, file) = setup_vault("tracked.md");
    sprout()
        .args(["init", file.to_str().unwrap(), "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already_initialized"));
}

#[test]
fn init_partial_adds_missing_fields() {
    let (dir, file) = setup_vault("partial.md");
    sprout()
        .args(["init", file.to_str().unwrap(), "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"fields_added\""))
        .stderr(predicate::str::contains("warning: missing fields added with defaults"));

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("maturity: budding")); // existing value preserved
    assert!(content.contains("ease: 2.50")); // existing value preserved
    assert!(content.contains("created:")); // added
    assert!(content.contains("review_interval:")); // added
}

#[test]
fn init_file_not_found() {
    let dir = TempDir::new().unwrap();
    sprout()
        .args(["init", "/nonexistent/file.md", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("file_not_found"));
}

// ── show ───────────────────────────────────────────────────────────

#[test]
fn show_tracked_note() {
    let (dir, file) = setup_vault("tracked.md");
    sprout()
        .args(["show", file.to_str().unwrap(), "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"tracked\":true"))
        .stdout(predicate::str::contains("\"maturity\":\"seedling\""))
        .stdout(predicate::str::contains("\"link_count\":2"))
        .stdout(predicate::str::contains("\"is_due\":"));
}

#[test]
fn show_untracked_note() {
    let (dir, file) = setup_vault("untracked.md");
    sprout()
        .args(["show", file.to_str().unwrap(), "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"tracked\":false"));
}

#[test]
fn show_file_not_found() {
    let dir = TempDir::new().unwrap();
    sprout()
        .args(["show", "/nonexistent.md", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("file_not_found"));
}

// ── list ───────────────────────────────────────────────────────────

#[test]
fn list_shows_tracked_notes() {
    let dir = setup_vault_multi(&["tracked.md", "untracked.md", "no_frontmatter.md"]);
    sprout()
        .args(["list", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked.md"))
        .stdout(predicate::str::contains("seedling"));

    // Should not include untracked or no_frontmatter
    let output = sprout()
        .args(["list", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("untracked.md"));
    assert!(!stdout.contains("no_frontmatter.md"));
}

#[test]
fn list_maturity_filter() {
    let dir = setup_vault_multi(&["tracked.md", "partial.md"]);
    // partial.md has maturity: budding but missing other fields → still tracked
    // First init partial to make it fully tracked
    let output = sprout()
        .args(["list", "--vault", dir.path().to_str().unwrap(), "--maturity", "budding", "--format", "json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("partial.md"));
    assert!(!stdout.contains("tracked.md")); // tracked.md is seedling, not budding
}

// ── review ─────────────────────────────────────────────────────────

#[test]
fn review_shows_due_notes() {
    // tracked.md has next_review: 2026-02-21, which is in the past
    let dir = setup_vault_multi(&["tracked.md", "untracked.md"]);
    sprout()
        .args(["review", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tracked.md"));
}

// ── stats ──────────────────────────────────────────────────────────

#[test]
fn stats_returns_correct_counts() {
    let dir = setup_vault_multi(&["tracked.md", "untracked.md", "partial.md"]);
    sprout()
        .args(["stats", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total\":2")) // tracked.md + partial.md
        .stdout(predicate::str::contains("\"seedling\":1"))
        .stdout(predicate::str::contains("\"budding\":1"));
}

// ── done ───────────────────────────────────────────────────────────

#[test]
fn done_updates_frontmatter() {
    let (dir, file) = setup_vault("tracked.md");
    sprout()
        .args(["done", file.to_str().unwrap(), "good", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"new_interval\":"))
        .stdout(predicate::str::contains("\"ease\":"));

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("last_review:"));
}

#[test]
fn done_no_frontmatter_errors() {
    let (dir, file) = setup_vault("untracked.md");
    sprout()
        .args(["done", file.to_str().unwrap(), "good", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no_frontmatter"));
}

#[test]
fn done_file_not_found() {
    let dir = TempDir::new().unwrap();
    sprout()
        .args(["done", "/nonexistent.md", "good", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("file_not_found"));
}

// ── promote ────────────────────────────────────────────────────────

#[test]
fn promote_changes_maturity() {
    let (dir, file) = setup_vault("tracked.md");
    sprout()
        .args(["promote", file.to_str().unwrap(), "budding", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"previous_maturity\":\"seedling\""))
        .stdout(predicate::str::contains("\"new_maturity\":\"budding\""));

    let content = fs::read_to_string(&file).unwrap();
    assert!(content.contains("maturity: budding"));
}

#[test]
fn promote_same_maturity_noop() {
    let (dir, file) = setup_vault("tracked.md");
    sprout()
        .args(["promote", file.to_str().unwrap(), "seedling", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"previous_maturity\":\"seedling\""))
        .stdout(predicate::str::contains("\"new_maturity\":\"seedling\""));
}

#[test]
fn promote_no_frontmatter_errors() {
    let (dir, file) = setup_vault("untracked.md");
    sprout()
        .args(["promote", file.to_str().unwrap(), "budding", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no_frontmatter"));
}

// ── error output ───────────────────────────────────────────────────

#[test]
fn error_json_format_on_stderr() {
    let dir = TempDir::new().unwrap();
    let output = sprout()
        .args(["show", "/nonexistent.md", "--vault", dir.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();

    // stdout should be empty on error
    assert!(output.stdout.is_empty(), "stdout should be empty on error");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("\"error\":\"file_not_found\""));
    assert!(stderr.contains("\"message\":"));
}

#[test]
fn error_exit_code_is_one() {
    let dir = TempDir::new().unwrap();
    sprout()
        .args(["show", "/nonexistent.md", "--vault", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .code(1);
}
