use crate::config::Config;
use crate::paths::AppPaths;
use crate::registry::{self, RegistryError};
use crate::session::Session;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

/// Shared, `Mutex`-guarded application state injected into IPC commands via Tauri's
/// `State<AppState>`. `rusqlite::Connection` is `Send + !Sync`, so every field holding one
/// must be wrapped in a `Mutex` before it can be shared across Tauri's async command
/// threadpool.
///
/// `vault` is `pub` (not `pub(crate)`) so that both `src-tauri` and `viewer-src-tauri` - separate
/// crates depending on this one - can lock/read it directly, the same way they already lock
/// `registry`/`session`/`config` directly. No new invariant is weakened: this field was already
/// freely accessible to every module within the one crate that used to own it.
pub struct AppState {
    pub registry: Mutex<Connection>,
    pub vault: Mutex<Option<(Connection, PathBuf)>>,
    pub session: Mutex<Session>,
    pub config: Mutex<Config>,
}

impl AppState {
    /// The one production constructor, shared by all three binaries (`src-tauri`,
    /// `viewer-src-tauri`, `web-src-tauri`) - previously duplicated inline at each call site.
    /// Loads `config.json` (defaulting with a stderr warning if it's missing/unreadable) and opens
    /// the on-disk registry connection at `paths.registry_db_path()`; the only failure mode a
    /// caller needs to handle is the registry open itself.
    pub fn open(paths: &AppPaths) -> Result<Self, RegistryError> {
        let config = Config::load(&paths.config_path()).unwrap_or_else(|e| {
            eprintln!("warning: failed to load config.json ({e}), using defaults");
            Config::default()
        });
        let timeout = Duration::from_secs(config.auto_lock_minutes as u64 * 60);
        let registry = registry::open_registry(&paths.registry_db_path())?;
        Ok(AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(Session::new(timeout)),
            config: Mutex::new(config),
        })
    }

    /// Test-only convenience constructor using an in-memory registry connection.
    #[cfg(test)]
    pub(crate) fn new(config: Config, timeout: Duration) -> Self {
        AppState {
            registry: Mutex::new(Connection::open_in_memory().unwrap()),
            vault: Mutex::new(None),
            session: Mutex::new(Session::new(timeout)),
            config: Mutex::new(config),
        }
    }

    /// Opens a vault connection and its on-disk path together, so a reader can never observe
    /// one half without the other.
    pub fn set_vault(&self, conn: Connection, path: PathBuf) {
        *self.vault.lock().unwrap() = Some((conn, path));
    }

    /// Closes the vault connection (if any) and clears its path in the same step.
    pub fn clear_vault(&self) {
        *self.vault.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::time::Duration;

    #[test]
    fn new_app_state_starts_with_no_vault_open_and_a_locked_session() {
        let state = AppState::new(Config::default(), Duration::from_secs(300));

        assert!(state.vault.lock().unwrap().is_none());
        assert!(state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn open_creates_a_locked_state_with_a_working_registry_connection() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();

        let state = AppState::open(&paths).unwrap();

        assert!(state.vault.lock().unwrap().is_none());
        assert!(state.session.lock().unwrap().is_locked());
        // Registry connection actually works (opened the real on-disk db, not a stand-in).
        state.registry.lock().unwrap().execute_batch("SELECT 1;").unwrap();
    }

    #[test]
    fn open_defaults_config_when_config_json_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();

        let state = AppState::open(&paths).unwrap();

        assert_eq!(state.config.lock().unwrap().auto_lock_minutes, Config::default().auto_lock_minutes);
    }

    #[test]
    fn set_vault_stores_connection_and_path() {
        let state = AppState::new(Config::default(), Duration::from_secs(300));
        let path = PathBuf::from("/tmp/example.vault");

        state.set_vault(Connection::open_in_memory().unwrap(), path.clone());

        let guard = state.vault.lock().unwrap();
        let (_, stored_path) = guard.as_ref().expect("vault should be set");
        assert_eq!(stored_path, &path);
    }

    #[test]
    fn clear_vault_resets_to_none() {
        let state = AppState::new(Config::default(), Duration::from_secs(300));
        state.set_vault(
            Connection::open_in_memory().unwrap(),
            PathBuf::from("/tmp/example.vault"),
        );

        state.clear_vault();

        assert!(state.vault.lock().unwrap().is_none());
    }
}
