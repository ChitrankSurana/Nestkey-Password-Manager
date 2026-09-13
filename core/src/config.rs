use serde::{Deserialize, Serialize};
use std::path::Path;

pub const VALID_THEMES: [&str; 2] = ["light", "dark"];
pub const VALID_AUTO_LOCK_MINUTES: [u32; 3] = [1, 5, 15];
pub const VALID_STRENGTH_ALGORITHMS: [&str; 2] = ["simple", "zxcvbn"];
pub const VALID_ACCENT_COLORS: [&str; 5] = ["indigo", "sky", "teal", "purple", "pink"];
pub const VALID_OLD_PASSWORD_DAYS: [u32; 3] = [90, 180, 365];
pub const VALID_MOBILE_NAV_STYLES: [&str; 3] = ["icon-rail", "bottom-tabs", "drawer"];

fn default_theme() -> String {
    "dark".to_string()
}

fn default_auto_lock_minutes() -> u32 {
    5
}

fn default_strength_algorithm() -> String {
    "simple".to_string()
}

fn default_accent_color() -> String {
    "indigo".to_string()
}

fn default_old_password_days() -> u32 {
    365
}

fn default_hibp_opt_in() -> bool {
    false
}

fn default_nav_collapsed() -> bool {
    false
}

fn default_mobile_nav_style() -> String {
    "icon-rail".to_string()
}

fn default_biometric_enabled() -> bool {
    false
}

fn default_ssh_agent_enabled() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_auto_lock_minutes")]
    pub auto_lock_minutes: u32,
    #[serde(default = "default_strength_algorithm")]
    pub strength_algorithm: String,
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
    #[serde(default = "default_old_password_days")]
    pub old_password_days: u32,
    #[serde(default = "default_hibp_opt_in")]
    pub hibp_opt_in: bool,
    #[serde(default = "default_nav_collapsed")]
    pub nav_collapsed: bool,
    #[serde(default = "default_mobile_nav_style")]
    pub mobile_nav_style: String,
    // Master on/off switch for the whole biometric-unlock feature - a safety valve independent of
    // whether any particular user has actually set it up (see has_pin/`pin_unlock` for the
    // per-user analog). Defaults off since it's new, platform-specific, and this project has no
    // way to verify Windows Hello + DPAPI integration against real hardware.
    #[serde(default = "default_biometric_enabled")]
    pub biometric_enabled: bool,
    // Same rationale as biometric_enabled: new native OS integration (a Windows named-pipe
    // ssh-agent server) this project can't verify against a real SSH client, defaulted off with
    // an explicit opt-in switch.
    #[serde(default = "default_ssh_agent_enabled")]
    pub ssh_agent_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: default_theme(),
            auto_lock_minutes: default_auto_lock_minutes(),
            strength_algorithm: default_strength_algorithm(),
            accent_color: default_accent_color(),
            old_password_days: default_old_password_days(),
            hibp_opt_in: default_hibp_opt_in(),
            nav_collapsed: default_nav_collapsed(),
            mobile_nav_style: default_mobile_nav_style(),
            biometric_enabled: default_biometric_enabled(),
            ssh_agent_enabled: default_ssh_agent_enabled(),
        }
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
        let parsed = if !VALID_THEMES.contains(&parsed.theme.as_str()) {
            Config { theme: default_theme(), ..parsed }
        } else {
            parsed
        };
        let parsed = if !VALID_AUTO_LOCK_MINUTES.contains(&parsed.auto_lock_minutes) {
            Config { auto_lock_minutes: default_auto_lock_minutes(), ..parsed }
        } else {
            parsed
        };
        let parsed = if !VALID_STRENGTH_ALGORITHMS.contains(&parsed.strength_algorithm.as_str()) {
            Config { strength_algorithm: default_strength_algorithm(), ..parsed }
        } else {
            parsed
        };
        let parsed = if !VALID_ACCENT_COLORS.contains(&parsed.accent_color.as_str()) {
            Config { accent_color: default_accent_color(), ..parsed }
        } else {
            parsed
        };
        let parsed = if !VALID_OLD_PASSWORD_DAYS.contains(&parsed.old_password_days) {
            Config { old_password_days: default_old_password_days(), ..parsed }
        } else {
            parsed
        };
        let parsed = if !VALID_MOBILE_NAV_STYLES.contains(&parsed.mobile_nav_style.as_str()) {
            Config { mobile_nav_style: default_mobile_nav_style(), ..parsed }
        } else {
            parsed
        };
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
        let config = Config {
            theme: "dark".to_string(),
            auto_lock_minutes: 15,
            strength_algorithm: "zxcvbn".to_string(),
            accent_color: default_accent_color(),
            old_password_days: default_old_password_days(),
            hibp_opt_in: default_hibp_opt_in(),
            nav_collapsed: default_nav_collapsed(),
            mobile_nav_style: default_mobile_nav_style(),
            biometric_enabled: default_biometric_enabled(),
            ssh_agent_enabled: default_ssh_agent_enabled(),
        };

        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.auto_lock_minutes, 15);
        assert_eq!(loaded.strength_algorithm, "zxcvbn");
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

    #[test]
    fn load_defaults_strength_algorithm_to_simple() {
        let dir = tempdir().unwrap();
        let config = Config::load(&dir.path().join("config.json")).unwrap();
        assert_eq!(config.strength_algorithm, "simple");
    }

    #[test]
    fn load_rejects_invalid_strength_algorithm_by_falling_back_to_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme":"dark","auto_lock_minutes":5,"strength_algorithm":"bogus"}"#).unwrap();
        assert_eq!(Config::load(&path).unwrap().strength_algorithm, "simple");
    }

    #[test]
    fn load_defaults_new_m4_fields() {
        let dir = tempdir().unwrap();
        let config = Config::load(&dir.path().join("config.json")).unwrap();
        assert_eq!(config.accent_color, "indigo");
        assert_eq!(config.old_password_days, 365);
        assert_eq!(config.hibp_opt_in, false);
        assert_eq!(config.nav_collapsed, false);
    }

    #[test]
    fn save_then_load_roundtrips_new_m4_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = Config {
            theme: "light".to_string(),
            auto_lock_minutes: 5,
            strength_algorithm: "simple".to_string(),
            accent_color: "teal".to_string(),
            old_password_days: 90,
            hibp_opt_in: true,
            nav_collapsed: true,
            mobile_nav_style: default_mobile_nav_style(),
            biometric_enabled: default_biometric_enabled(),
            ssh_agent_enabled: default_ssh_agent_enabled(),
        };

        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.accent_color, "teal");
        assert_eq!(loaded.old_password_days, 90);
        assert_eq!(loaded.hibp_opt_in, true);
        assert_eq!(loaded.nav_collapsed, true);
    }

    #[test]
    fn load_rejects_invalid_theme_by_falling_back_to_default() {
        // theme predates M4 but never got the same load-time sanitization the other
        // string/enum-like fields did - a hand-edited config.json with a bogus theme must not
        // flow an unvalidated value through to the frontend, same guarantee as accent_color.
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme":"bogus"}"#).unwrap();
        assert_eq!(Config::load(&path).unwrap().theme, "dark");
    }

    #[test]
    fn load_rejects_invalid_accent_color_by_falling_back_to_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"accent_color":"bogus"}"#).unwrap();
        assert_eq!(Config::load(&path).unwrap().accent_color, "indigo");
    }

    #[test]
    fn load_defaults_mobile_nav_style_to_icon_rail() {
        let dir = tempdir().unwrap();
        let config = Config::load(&dir.path().join("config.json")).unwrap();
        assert_eq!(config.mobile_nav_style, "icon-rail");
    }

    #[test]
    fn save_then_load_roundtrips_mobile_nav_style() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = Config { mobile_nav_style: "bottom-tabs".to_string(), ..Config::default() };

        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();

        assert_eq!(loaded.mobile_nav_style, "bottom-tabs");
    }

    #[test]
    fn load_rejects_invalid_mobile_nav_style_by_falling_back_to_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"mobile_nav_style":"sidebar-carousel"}"#).unwrap();
        assert_eq!(Config::load(&path).unwrap().mobile_nav_style, "icon-rail");
    }

    #[test]
    fn load_rejects_invalid_old_password_days_by_falling_back_to_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"old_password_days":999}"#).unwrap();
        assert_eq!(Config::load(&path).unwrap().old_password_days, 365);
    }

    #[test]
    fn load_sanitizes_every_invalid_field_at_once_not_just_the_first() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(
            &path,
            r#"{"auto_lock_minutes":999,"strength_algorithm":"bogus","accent_color":"bogus","old_password_days":999}"#,
        )
        .unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.auto_lock_minutes, 5);
        assert_eq!(config.strength_algorithm, "simple");
        assert_eq!(config.accent_color, "indigo");
        assert_eq!(config.old_password_days, 365);
    }

    #[test]
    fn load_preserves_present_new_fields_when_another_field_is_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"hibp_opt_in":true}"#).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.hibp_opt_in, true);
        assert_eq!(config.accent_color, "indigo"); // untouched field still defaults correctly
    }
}
