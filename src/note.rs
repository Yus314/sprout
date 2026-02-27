use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::error::SproutError;
use crate::frontmatter::{parse_note, ParsedNote};

pub struct NoteInfo {
    pub path: PathBuf,
    pub relative_path: String,
    pub parsed: ParsedNote,
}

pub struct NotePath {
    pub path: PathBuf,
    pub relative_path: String,
}

struct MdEntry {
    canonical: PathBuf,
    relative: String,
}

/// Walk the vault and collect canonical+relative paths for all `.md` files.
/// Symlinks are followed; cycles are skipped. Duplicate paths (via canonicalize) are deduplicated.
fn collect_md_paths(vault: &Path, exclude_dirs: &[String]) -> Result<Vec<MdEntry>> {
    let vault_canonical = std::fs::canonicalize(vault)
        .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;

    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    let walker = WalkDir::new(&vault_canonical)
        .follow_links(true)
        .into_iter();

    for entry in walker.filter_entry(|e| {
        if e.file_type().is_dir() {
            let name = e.file_name().to_string_lossy();
            !exclude_dirs.iter().any(|d| d == name.as_ref())
        } else {
            true
        }
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // skip cycle errors, permission errors, etc.
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let canonical = match std::fs::canonicalize(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !seen.insert(canonical.clone()) {
            continue; // duplicate
        }

        let relative = canonical
            .strip_prefix(&vault_canonical)
            .unwrap_or(&canonical)
            .to_string_lossy()
            .to_string();

        entries.push(MdEntry { canonical, relative });
    }

    Ok(entries)
}

/// Scan the vault for `.md` file paths only (no file content is read).
/// Symlinks are followed; cycles are skipped. Duplicate paths (via canonicalize) are deduplicated.
pub fn scan_vault_paths(vault: &Path, exclude_dirs: &[String]) -> Result<Vec<NotePath>> {
    let entries = collect_md_paths(vault, exclude_dirs)?;
    Ok(entries
        .into_iter()
        .map(|e| NotePath {
            path: e.canonical,
            relative_path: e.relative,
        })
        .collect())
}

/// Scan the vault for .md files, excluding specified directories.
/// Symlinks are followed; cycles are skipped. Duplicate paths (via canonicalize) are deduplicated.
/// Files that cannot be read are warned to stderr and skipped.
pub fn scan_vault(vault: &Path, exclude_dirs: &[String]) -> Result<Vec<NoteInfo>> {
    let entries = collect_md_paths(vault, exclude_dirs)?;
    let mut notes = Vec::new();

    for entry in entries {
        let content = match std::fs::read_to_string(&entry.canonical) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "warning: skipping {}: {}",
                    entry.canonical.display(),
                    e
                );
                continue;
            }
        };

        let parsed = parse_note(&content);

        notes.push(NoteInfo {
            path: entry.canonical,
            relative_path: entry.relative,
            parsed,
        });
    }

    Ok(notes)
}

/// Read and parse a single note file.
pub fn read_note(path: &Path) -> Result<ParsedNote, SproutError> {
    let content = std::fs::read_to_string(path)
        .map_err(|_| SproutError::FileNotFound(path.display().to_string()))?;
    Ok(parse_note(&content))
}

/// Write content to a note file.
pub fn write_note(path: &Path, content: &str) -> Result<(), SproutError> {
    std::fs::write(path, content)
        .map_err(|e| SproutError::ParseError(format!("failed to write {}: {e}", path.display())))
}

/// Ensure the file is inside the vault directory.
/// Both paths are canonicalized before comparison.
pub fn ensure_in_vault(file: &Path, vault: &Path) -> Result<(), SproutError> {
    let file_canonical = std::fs::canonicalize(file)
        .map_err(|_| SproutError::FileNotFound(file.display().to_string()))?;
    let vault_canonical = std::fs::canonicalize(vault)
        .map_err(|_| SproutError::VaultNotFound(vault.display().to_string()))?;

    if file_canonical.starts_with(&vault_canonical) {
        Ok(())
    } else {
        Err(SproutError::OutsideVault(
            file_canonical.display().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_read_note_valid() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        fs::write(&file, "---\nmaturity: seedling\n---\nHello\n").unwrap();

        let parsed = read_note(&file).unwrap();
        assert_eq!(parsed.sprout.maturity.as_deref(), Some("seedling"));
        assert!(parsed.body.contains("Hello"));
    }

    #[test]
    fn test_read_note_nonexistent() {
        let result = read_note(Path::new("/nonexistent/path.md"));
        assert!(result.is_err());
        match result.unwrap_err() {
            SproutError::FileNotFound(_) => {}
            other => panic!("expected FileNotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_write_note_roundtrip() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        let content = "---\nmaturity: seedling\n---\nBody\n";
        write_note(&file, content).unwrap();

        let read_back = fs::read_to_string(&file).unwrap();
        assert_eq!(read_back, content);
    }

    #[test]
    fn test_ensure_in_vault_inside() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("note.md");
        fs::write(&file, "test").unwrap();

        assert!(ensure_in_vault(&file, dir.path()).is_ok());
    }

    #[test]
    fn test_ensure_in_vault_outside() {
        let vault = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let file = outside.path().join("note.md");
        fs::write(&file, "test").unwrap();

        let result = ensure_in_vault(&file, vault.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            SproutError::OutsideVault(_) => {}
            other => panic!("expected OutsideVault, got {other:?}"),
        }
    }

    #[test]
    fn test_scan_vault_finds_md_files() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("tracked.md"),
            "---\nmaturity: seedling\n---\nBody\n",
        )
        .unwrap();
        fs::write(dir.path().join("plain.md"), "No frontmatter\n").unwrap();
        fs::write(dir.path().join("not_md.txt"), "ignored").unwrap();

        let notes = scan_vault(dir.path(), &[]).unwrap();
        assert_eq!(notes.len(), 2); // only .md files
        let paths: Vec<_> = notes.iter().map(|n| n.relative_path.as_str()).collect();
        assert!(paths.contains(&"tracked.md"));
        assert!(paths.contains(&"plain.md"));
    }

    #[test]
    fn test_scan_vault_excludes_dirs() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("root.md"), "---\nmaturity: seedling\n---\n").unwrap();
        let subdir = dir.path().join(".obsidian");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("hidden.md"), "---\nmaturity: seedling\n---\n").unwrap();

        let notes = scan_vault(dir.path(), &[".obsidian".into()]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].relative_path, "root.md");
    }

    #[test]
    fn test_scan_vault_subdirectory() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("deep.md"), "---\nmaturity: budding\n---\n").unwrap();

        let notes = scan_vault(dir.path(), &[]).unwrap();
        assert_eq!(notes.len(), 1);
        assert!(notes[0].relative_path.contains("deep.md"));
    }

    #[test]
    fn test_scan_vault_nonexistent() {
        let result = scan_vault(Path::new("/nonexistent/vault"), &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_vault_paths_finds_md_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.md"), "content").unwrap();
        fs::write(dir.path().join("b.md"), "content").unwrap();
        fs::write(dir.path().join("c.txt"), "content").unwrap();

        let paths = scan_vault_paths(dir.path(), &[]).unwrap();
        assert_eq!(paths.len(), 2);
        let rel: Vec<_> = paths.iter().map(|p| p.relative_path.as_str()).collect();
        assert!(rel.contains(&"a.md"));
        assert!(rel.contains(&"b.md"));
    }

    #[test]
    fn test_scan_vault_paths_excludes_dirs() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("root.md"), "content").unwrap();
        let hidden = dir.path().join(".obsidian");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("hidden.md"), "content").unwrap();

        let paths = scan_vault_paths(dir.path(), &[".obsidian".into()]).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].relative_path, "root.md");
    }
}
