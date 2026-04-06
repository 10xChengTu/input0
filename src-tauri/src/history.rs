use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config;
use crate::errors::AppError;
use crate::llm::client::HistoryEntry;

const HISTORY_FILENAME: &str = "history.json";
const MAX_HISTORY_ENTRIES: usize = 10;

static HISTORY_LOCK: Mutex<()> = Mutex::new(());

/// Load transcription history from the default config directory.
/// Never errors — returns an empty Vec on any failure (missing file, corrupt JSON, etc.).
pub fn load_history() -> Vec<HistoryEntry> {
    match config::config_dir() {
        Ok(dir) => load_from_dir(&dir),
        Err(_) => Vec::new(),
    }
}

/// Save transcription history to the default config directory.
/// Caps entries at MAX_HISTORY_ENTRIES, keeping the most recent ones.
pub fn save_history(entries: &[HistoryEntry]) -> Result<(), AppError> {
    save_to_dir(entries, &config::config_dir()?)
}

/// Atomically load history, append one entry, and save back.
/// Uses a process-level mutex to prevent concurrent read-modify-write races.
pub fn append_entry(entry: HistoryEntry) -> Result<(), AppError> {
    let _guard = HISTORY_LOCK.try_lock().map_err(|e| {
        AppError::Config(format!("History lock contention (try_lock failed): {}", e))
    })?;
    let mut entries = load_history();
    entries.push(entry);
    save_history(&entries)
}

fn history_path(dir: &Path) -> PathBuf {
    dir.join(HISTORY_FILENAME)
}

fn load_from_dir(dir: &Path) -> Vec<HistoryEntry> {
    let path = history_path(dir);
    if !path.exists() {
        return Vec::new();
    }
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read history file {}: {}", path.display(), e);
            return Vec::new();
        }
    };
    if contents.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<HistoryEntry>>(&contents) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Corrupt history file {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

fn save_to_dir(entries: &[HistoryEntry], dir: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dir).map_err(|e| AppError::Io(e))?;

    let capped: &[HistoryEntry] = if entries.len() > MAX_HISTORY_ENTRIES {
        &entries[entries.len() - MAX_HISTORY_ENTRIES..]
    } else {
        entries
    };

    let json = serde_json::to_string_pretty(capped)
        .map_err(|e| AppError::Config(format!("Failed to serialize history: {}", e)))?;

    // Atomic write: write to a temp file first, then rename to the target path.
    // This prevents data loss if the process crashes mid-write.
    let target = history_path(dir);
    let tmp_path = dir.join(".history.json.tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| AppError::Io(e))?;
    std::fs::rename(&tmp_path, &target).map_err(|e| AppError::Io(e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(original: &str, corrected: &str) -> HistoryEntry {
        HistoryEntry {
            original: original.to_string(),
            corrected: corrected.to_string(),
        }
    }

    #[test]
    fn test_load_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let entries = load_from_dir(tmp.path());
        assert!(entries.is_empty());
    }

    #[test]
    fn test_roundtrip_single_entry() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![make_entry("你好", "你好")];
        save_to_dir(&entries, tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].original, "你好");
        assert_eq!(loaded[0].corrected, "你好");
    }

    #[test]
    fn test_roundtrip_multiple_entries() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![
            make_entry("瑞嗯特", "React"),
            make_entry("诶辟爱", "API"),
            make_entry("杰森数据", "JSON 数据"),
        ];
        save_to_dir(&entries, tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].corrected, "React");
        assert_eq!(loaded[1].corrected, "API");
        assert_eq!(loaded[2].corrected, "JSON 数据");
    }

    #[test]
    fn test_caps_at_max_entries() {
        let tmp = TempDir::new().unwrap();
        let entries: Vec<HistoryEntry> = (0..15)
            .map(|i| make_entry(&format!("original_{}", i), &format!("corrected_{}", i)))
            .collect();
        save_to_dir(&entries, tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert_eq!(loaded.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(loaded[0].original, "original_5");
        assert_eq!(loaded[9].original, "original_14");
    }

    #[test]
    fn test_exactly_max_entries() {
        let tmp = TempDir::new().unwrap();
        let entries: Vec<HistoryEntry> = (0..MAX_HISTORY_ENTRIES)
            .map(|i| make_entry(&format!("orig_{}", i), &format!("corr_{}", i)))
            .collect();
        save_to_dir(&entries, tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert_eq!(loaded.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(loaded[0].original, "orig_0");
    }

    #[test]
    fn test_corrupt_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(HISTORY_FILENAME);
        std::fs::write(&path, "this is not valid json!!!").unwrap();
        let loaded = load_from_dir(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_empty_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(HISTORY_FILENAME);
        std::fs::write(&path, "").unwrap();
        let loaded = load_from_dir(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_whitespace_only_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(HISTORY_FILENAME);
        std::fs::write(&path, "   \n  \t  ").unwrap();
        let loaded = load_from_dir(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_overwrite_existing() {
        let tmp = TempDir::new().unwrap();

        let first = vec![make_entry("first", "first_corrected")];
        save_to_dir(&first, tmp.path()).unwrap();
        assert_eq!(load_from_dir(tmp.path()).len(), 1);

        let second = vec![make_entry("a", "b"), make_entry("c", "d")];
        save_to_dir(&second, tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].original, "a");
        assert_eq!(loaded[1].original, "c");
    }

    #[test]
    fn test_empty_json_array_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(HISTORY_FILENAME);
        std::fs::write(&path, "[]").unwrap();
        let loaded = load_from_dir(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_save_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("nested").join("dir");
        let entries = vec![make_entry("test", "test")];
        save_to_dir(&entries, &nested).unwrap();
        let loaded = load_from_dir(&nested);
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_save_empty_vec() {
        let tmp = TempDir::new().unwrap();
        save_to_dir(&[], tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_unicode_content_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let entries = vec![
            make_entry("我在用瑞嗯特写组件", "我在用 React 写组件"),
            make_entry("用泰普斯克瑞普特开发", "用 TypeScript 开发"),
        ];
        save_to_dir(&entries, tmp.path()).unwrap();
        let loaded = load_from_dir(tmp.path());
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].corrected, "我在用 React 写组件");
        assert_eq!(loaded[1].corrected, "用 TypeScript 开发");
    }
}
