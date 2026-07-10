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
    pub vault: Mutex<Option<Connection>>,
    pub vault_path: Mutex<Option<PathBuf>>,
    pub session: Mutex<Session>,
    pub config: Mutex<Config>,
}

impl AppState {
    /// Test-only convenience constructor using an in-memory registry connection. The real
    /// constructor used by main.rs (Task 9) opens the on-disk registry via AppPaths instead.
    #[cfg(test)]
    fn new(config: Config, timeout: Duration) -> Self {
        AppState {
            registry: Mutex::new(Connection::open_in_memory().unwrap()),
            vault: Mutex::new(None),
            vault_path: Mutex::new(None),
            session: Mutex::new(Session::new(timeout)),
            config: Mutex::new(config),
        }
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
        assert!(state.vault_path.lock().unwrap().is_none());
    }
}
