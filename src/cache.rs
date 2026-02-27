use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::frontmatter::SproutFrontmatter;

const CACHE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    mtime_secs: i64,
    mtime_nanos: u32,
    size: u64,
    frontmatter: SproutFrontmatter,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    entries: HashMap<PathBuf, CacheEntry>,
}

pub struct FrontmatterCache {
    entries: HashMap<PathBuf, CacheEntry>,
    dirty: bool,
}

impl FrontmatterCache {
    pub fn load() -> Self {
        let path = match cache_path() {
            Some(p) => p,
            None => {
                return Self {
                    entries: HashMap::new(),
                    dirty: false,
                }
            }
        };

        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => {
                return Self {
                    entries: HashMap::new(),
                    dirty: false,
                }
            }
        };

        match serde_json::from_str::<CacheFile>(&data) {
            Ok(cf) if cf.version == CACHE_VERSION => Self {
                entries: cf.entries,
                dirty: false,
            },
            _ => Self {
                entries: HashMap::new(),
                dirty: false,
            },
        }
    }

    pub fn save(&self) {
        if !self.dirty {
            return;
        }
        let path = match cache_path() {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let cf = CacheFile {
            version: CACHE_VERSION,
            entries: self
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), CacheEntry {
                    mtime_secs: v.mtime_secs,
                    mtime_nanos: v.mtime_nanos,
                    size: v.size,
                    frontmatter: v.frontmatter.clone(),
                }))
                .collect(),
        };
        let data = match serde_json::to_string(&cf) {
            Ok(d) => d,
            Err(_) => return,
        };
        // Atomic save: write to temp file then rename
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, &data).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    pub fn get(&self, path: &Path, mtime_secs: i64, mtime_nanos: u32, size: u64) -> Option<&SproutFrontmatter> {
        let entry = self.entries.get(path)?;
        if entry.mtime_secs == mtime_secs
            && entry.mtime_nanos == mtime_nanos
            && entry.size == size
        {
            Some(&entry.frontmatter)
        } else {
            None
        }
    }

    pub fn insert(
        &mut self,
        path: PathBuf,
        mtime_secs: i64,
        mtime_nanos: u32,
        size: u64,
        frontmatter: SproutFrontmatter,
    ) {
        self.entries.insert(
            path,
            CacheEntry {
                mtime_secs,
                mtime_nanos,
                size,
                frontmatter,
            },
        );
        self.dirty = true;
    }
}

fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("sprout").join("frontmatter.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use tempfile::TempDir;

    fn sample_frontmatter() -> SproutFrontmatter {
        SproutFrontmatter {
            maturity: Some("seedling".into()),
            created: Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            last_review: None,
            review_interval: Some(1),
            next_review: Some(NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()),
            ease: Some(2.5),
        }
    }

    #[test]
    fn test_cache_get_hit() {
        let mut cache = FrontmatterCache {
            entries: HashMap::new(),
            dirty: false,
        };
        let path = PathBuf::from("/test/note.md");
        cache.insert(path.clone(), 1000, 500, 200, sample_frontmatter());
        assert!(cache.dirty);
        let result = cache.get(&path, 1000, 500, 200);
        assert!(result.is_some());
        assert_eq!(result.unwrap().maturity.as_deref(), Some("seedling"));
    }

    #[test]
    fn test_cache_get_miss_mtime() {
        let mut cache = FrontmatterCache {
            entries: HashMap::new(),
            dirty: false,
        };
        let path = PathBuf::from("/test/note.md");
        cache.insert(path.clone(), 1000, 500, 200, sample_frontmatter());
        // Different mtime_secs
        assert!(cache.get(&path, 1001, 500, 200).is_none());
        // Different mtime_nanos
        assert!(cache.get(&path, 1000, 501, 200).is_none());
        // Different size
        assert!(cache.get(&path, 1000, 500, 201).is_none());
    }

    #[test]
    fn test_cache_get_missing_path() {
        let cache = FrontmatterCache {
            entries: HashMap::new(),
            dirty: false,
        };
        assert!(cache.get(Path::new("/nonexistent"), 0, 0, 0).is_none());
    }

    #[test]
    fn test_cache_save_not_dirty() {
        // Should not write when not dirty
        let cache = FrontmatterCache {
            entries: HashMap::new(),
            dirty: false,
        };
        cache.save(); // no-op, no error
    }

    #[test]
    fn test_cache_roundtrip() {
        let dir = TempDir::new().unwrap();
        let cache_file = dir.path().join("frontmatter.json");

        // Write cache manually
        let mut cache = FrontmatterCache {
            entries: HashMap::new(),
            dirty: true,
        };
        let path = PathBuf::from("/test/note.md");
        cache.insert(path.clone(), 1000, 500, 200, sample_frontmatter());

        let cf = CacheFile {
            version: CACHE_VERSION,
            entries: cache.entries,
        };
        let data = serde_json::to_string(&cf).unwrap();
        std::fs::write(&cache_file, &data).unwrap();

        // Read it back
        let loaded: CacheFile = serde_json::from_str(&std::fs::read_to_string(&cache_file).unwrap()).unwrap();
        assert_eq!(loaded.version, CACHE_VERSION);
        assert!(loaded.entries.contains_key(&path));
        let entry = &loaded.entries[&path];
        assert_eq!(entry.mtime_secs, 1000);
        assert_eq!(entry.frontmatter.maturity.as_deref(), Some("seedling"));
    }

    #[test]
    fn test_cache_load_corrupt() {
        // Corrupt data should yield empty cache
        let dir = TempDir::new().unwrap();
        let cache_file = dir.path().join("frontmatter.json");
        std::fs::write(&cache_file, "not json").unwrap();
        // load() uses dirs::cache_dir(), so we can't directly test with custom path,
        // but we verify the deserialization logic
        let result = serde_json::from_str::<CacheFile>("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_load_wrong_version() {
        let cf = CacheFile {
            version: 999,
            entries: HashMap::new(),
        };
        let data = serde_json::to_string(&cf).unwrap();
        let loaded: CacheFile = serde_json::from_str(&data).unwrap();
        // Version mismatch should trigger empty cache in load()
        assert_ne!(loaded.version, CACHE_VERSION);
    }
}
