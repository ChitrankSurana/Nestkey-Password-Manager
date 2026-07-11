use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) const VALID_AUTO_LOCK_MINUTES: [u32; 3] = [1, 5, 15];

fn default_theme() -> String {
    "dark".to_string()
}

fn default_auto_lock_minutes() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_auto_lock_minutes")]
    pub auto_lock_minutes: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config { theme: default_theme(), auto_lock_minutes: default_auto_lock_minutes() }
    }
}

impl Config {
    pub fn load(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let raw = std::fs::read_to_string(path)?;
        // Per-field #[serde(default = ...)] means a config.json missing a field (e.g. saved by
        // an older app version before a new field existed) only loses that one field to its
        // default, not the whole struct - a parse error here means genuinely malformed content
        // (not valid JSON, or a field with the wrong type), which is rare enough to warrant a
        // diagnostic before falling back, same precedent as paths.rs's icacls best-effort warning.
        let parsed: Config = match serde_json::from_str(&raw) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("warning: config.json unreadable ({e}), resetting to defaults");
                Config::default()
            }
        };
        if !VALID_AUTO_LOCK_MINUTES.contains(&parsed.auto_lock_minutes) {
            return Ok(Config { auto_lock_minutes: default_auto_lock_minutes(), ..parsed });
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

    #[test]
    fn load_preserves_present_fields_when_another_field_is_missing() {
        // A config.json missing a field (e.g. saved by an older app version before a new field
        // was added) must NOT discard the fields that ARE present - only the missing field
        // should fall back to its own default.
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme":"light"}"#).unwrap();

        let config = Config::load(&path).unwrap();

        assert_eq!(config.theme, "light");
        assert_eq!(config.auto_lock_minutes, 5);
    }

    #[test]
    fn load_falls_back_to_default_on_malformed_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "not json").unwrap();

        let config = Config::load(&path).unwrap();

        assert_eq!(config, Config::default());
    }
}
