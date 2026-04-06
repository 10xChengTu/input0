use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config;
use crate::errors::AppError;

const VOCABULARY_FILENAME: &str = "vocabulary.json";
const MAX_VOCABULARY_ENTRIES: usize = 500;

static VOCABULARY_LOCK: Mutex<()> = Mutex::new(());

/// Load vocabulary from config dir. Never errors — returns empty Vec on failure.
pub fn load_vocabulary() -> Vec<String> {
    match config::config_dir() {
        Ok(dir) => load_from_dir(&dir),
        Err(_) => Vec::new(),
    }
}

/// Save vocabulary to config dir.
pub fn save_vocabulary(entries: &[String]) -> Result<(), AppError> {
    save_to_dir(entries, &config::config_dir()?)
}

/// Add a vocabulary term. Deduplicates by exact string match (case-sensitive).
/// Returns `true` if the term was newly added, `false` if it already existed.
pub fn add_entry(term: String) -> Result<bool, AppError> {
    let _guard = VOCABULARY_LOCK.try_lock().map_err(|e| {
        AppError::Config(format!(
            "Vocabulary lock contention (try_lock failed): {}",
            e
        ))
    })?;
    let mut entries = load_vocabulary();

    if entries.iter().any(|e| *e == term) {
        return Ok(false);
    }

    entries.push(term);
    save_vocabulary(&entries)?;
    Ok(true)
}

/// Remove a vocabulary term by exact match.
pub fn remove_entry(term: &str) -> Result<bool, AppError> {
    let _guard = VOCABULARY_LOCK.try_lock().map_err(|e| {
        AppError::Config(format!(
            "Vocabulary lock contention (try_lock failed): {}",
            e
        ))
    })?;
    let mut entries = load_vocabulary();

    let initial_len = entries.len();
    entries.retain(|e| e != term);

    if entries.len() < initial_len {
        save_vocabulary(&entries)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn vocabulary_path(dir: &Path) -> PathBuf {
    dir.join(VOCABULARY_FILENAME)
}

fn load_from_dir(dir: &Path) -> Vec<String> {
    let path = vocabulary_path(dir);
    if !path.exists() {
        return Vec::new();
    }
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read vocabulary file {}: {}", path.display(), e);
            return Vec::new();
        }
    };
    if contents.trim().is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<String>>(&contents) {
        Ok(entries) => entries,
        Err(e) => {
            log::warn!("Corrupt vocabulary file {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

fn save_to_dir(entries: &[String], dir: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dir).map_err(|e| AppError::Io(e))?;

    let capped: &[String] = if entries.len() > MAX_VOCABULARY_ENTRIES {
        &entries[entries.len() - MAX_VOCABULARY_ENTRIES..]
    } else {
        entries
    };

    let json = serde_json::to_string_pretty(capped)
        .map_err(|e| AppError::Config(format!("Failed to serialize vocabulary: {}", e)))?;

    // Atomic write: write to a temp file first, then rename to the target path.
    // This prevents data loss if the process crashes mid-write.
    let target = vocabulary_path(dir);
    let tmp_path = dir.join(".vocabulary.json.tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| AppError::Io(e))?;
    std::fs::rename(&tmp_path, &target).map_err(|e| AppError::Io(e))?;
    Ok(())
}
