use serde::{Deserialize, Serialize};
use std::path::Path;

const VALID_AUTO_LOCK_MINUTES: [u32; 3] = [1, 5, 15];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub theme: String,
    pub auto_lock_minutes: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config { theme: "dark".to_string(), auto_lock_minutes: 5 }
    }
}

impl Config {
    /// Loads config from `path`, falling back to `Config::default()` when the file is missing,
    /// unparseable, or has an out-of-range `auto_lock_minutes` - a hand-edited or corrupted
    /// config.json must never produce an invalid Session timeout.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let raw = std::fs::read_to_string(path)?;
        let parsed: Config = serde_json::from_str(&raw).unwrap_or_default();
        if !VALID_AUTO_LOCK_MINUTES.contains(&parsed.auto_lock_minutes) {
            return Ok(Config { auto_lock_minutes: Config::default().auto_lock_minutes, ..parsed });
        }
        Ok(parsed)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let raw = serde_json::to_string_pretty(self)?;
        std::fs::write(path, raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_returns_default_when_file_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = Config::load(&path).unwrap();

        assert_eq!(config.theme, "dark");
        assert_eq!(config.auto_lock_minutes, 5);
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = Config { theme: "dark".to_string(), auto_lock_minutes: 15 };

        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.auto_lock_minutes, 15);
    }

    #[test]
    fn load_rejects_invalid_auto_lock_minutes_by_falling_back_to_default() {
        // A hand-edited or corrupted config.json with an out-of-range value must not silently
        // become an invalid Session timeout (e.g. 0 minutes locking instantly, or a huge value
        // defeating auto-lock entirely) - fall back to the safe default instead.
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme":"dark","auto_lock_minutes":999}"#).unwrap();

        let config = Config::load(&path).unwrap();

        assert_eq!(config.auto_lock_minutes, 5);
    }
}
