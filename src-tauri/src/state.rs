use crate::config::Config;
use pwmgr_core::session::Session;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

/// Shared, `Mutex`-guarded application state injected into IPC commands via Tauri's
/// `State<AppState>`. `rusqlite::Connection` is `Send + !Sync`, so every field holding one
/// must be wrapped in a `Mutex` before it can be shared across Tauri's async command
/// threadpool.
pub struct AppState {
    pub registry: Mutex<Connection>,
    pub(crate) vault: Mutex<Option<(Connection, PathBuf)>>,
    pub session: Mutex<Session>,
    pub config: Mutex<Config>,
}

impl AppState {
    /// Test-only convenience constructor using an in-memory registry connection. The real
    /// constructor used by main.rs (Task 9) opens the on-disk registry via AppPaths instead.
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
