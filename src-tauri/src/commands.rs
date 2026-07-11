use crate::dto::AuthSuccess;
use crate::error::CommandError;
use crate::paths::AppPaths;
use crate::state::AppState;
use pwmgr_core::session::SystemClock;
use tauri::State;

/// Smoke-test command for Slice 0: proves the IPC round trip (state access, Mutex lock, DTO
/// return) works end-to-end before Slice 1 adds real auth commands on the same pattern.
#[tauri::command]
pub fn unlock_status(state: State<AppState>) -> bool {
    unlock_status_inner(&state)
}

fn unlock_status_inner(state: &AppState) -> bool {
    !state.session.lock().unwrap().is_locked()
}

pub fn do_register(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    master_password: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.vault_path(username);
    let key = {
        let registry = state.registry.lock().unwrap();
        pwmgr_core::auth::register_user(&registry, &vault_path, username, master_password)?
    };
    let conn = pwmgr_core::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
}

pub fn do_login(
    state: &AppState,
    paths: &AppPaths,
    username: &str,
    master_password: &str,
) -> Result<AuthSuccess, CommandError> {
    let vault_path = paths.vault_path(username);
    let key = {
        let registry = state.registry.lock().unwrap();
        pwmgr_core::auth::login(&registry, &vault_path, username, master_password)?
    };
    let conn = pwmgr_core::vault::open_vault(&vault_path, &key)?;
    state.session.lock().unwrap().unlock(key, &SystemClock);
    state.set_vault(conn, vault_path);
    Ok(AuthSuccess { username: username.to_string() })
}

pub fn do_lock(state: &AppState) {
    state.session.lock().unwrap().lock();
    state.clear_vault();
}

pub fn do_touch(state: &AppState) {
    state.session.lock().unwrap().touch(&SystemClock);
}

#[tauri::command]
pub fn register_user(
    state: State<AppState>,
    paths: State<AppPaths>,
    username: String,
    master_password: String,
) -> Result<AuthSuccess, CommandError> {
    do_register(&state, &paths, &username, &master_password)
}

#[tauri::command]
pub fn login(
    state: State<AppState>,
    paths: State<AppPaths>,
    username: String,
    master_password: String,
) -> Result<AuthSuccess, CommandError> {
    do_login(&state, &paths, &username, &master_password)
}

#[tauri::command]
pub fn lock(state: State<AppState>) {
    do_lock(&state);
}

#[tauri::command]
pub fn touch(state: State<AppState>) {
    do_touch(&state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use pwmgr_core::session::{Clock, Session};
    use std::cell::RefCell;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    // `autolock.rs` has its own `FakeClock` in its private `#[cfg(test)] mod tests`, which is not
    // reachable from here (the module itself isn't `pub`). Duplicating this ~10-line helper is
    // cheaper and clearer than inventing a shared test-support module for two call sites.
    struct FakeClock(RefCell<Instant>);
    impl FakeClock {
        fn new() -> Self {
            FakeClock(RefCell::new(Instant::now()))
        }
        fn advance(&self, d: Duration) {
            *self.0.borrow_mut() += d;
        }
    }
    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            *self.0.borrow()
        }
    }

    #[test]
    fn unlock_status_inner_reflects_locked_session() {
        let state = AppState::new(Config::default(), Duration::from_secs(300));
        assert!(!unlock_status_inner(&state));
    }

    fn test_state_and_paths() -> (AppState, AppPaths, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();
        let registry = pwmgr_core::registry::open_registry(&paths.registry_db_path()).unwrap();
        let state = AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(Session::new(Duration::from_secs(300))),
            config: Mutex::new(Config::default()),
        };
        (state, paths, dir)
    }

    #[test]
    fn register_then_login_both_unlock_the_session_and_open_the_vault() {
        let (state, paths, _dir) = test_state_and_paths();

        let reg = do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(reg.username, "alice");
        assert!(!state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_some());

        do_lock(&state);
        assert!(state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_none());

        let login = do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(login.username, "alice");
        assert!(!state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_some());
    }

    #[test]
    fn login_with_wrong_password_returns_single_generic_error_and_stays_locked() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        do_lock(&state);

        let result = do_login(&state, &paths, "alice", "wrong password");

        assert!(result.is_err());
        // CommandError's fields are private outside the `error` module by design (see error.rs) -
        // its derived Debug output is the only cross-module way to observe the code/message
        // without adding a new accessor purely for test visibility.
        let debug = format!("{:?}", result.unwrap_err());
        assert!(debug.contains("invalid_credentials"));
        assert!(state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_none());
    }

    #[test]
    fn register_duplicate_username_returns_username_taken_error() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "password one").unwrap();
        do_lock(&state);

        // A second AppPaths (different vaults dir, same `state.registry` connection) so this
        // test isolates the registry's UsernameTaken check. do_register derives vault_path
        // deterministically from the username, and pwmgr_core::auth::register_user calls
        // open_vault (which fails on an existing file keyed with a different, freshly-randomized
        // salt) BEFORE checking registry uniqueness - reusing the same `paths` for both calls
        // would hit that vault-path collision (a VaultError, mapped to "internal_error") instead
        // of the UsernameTaken error this test targets.
        let dir2 = tempfile::tempdir().unwrap();
        let paths2 = AppPaths::new(dir2.path().to_path_buf());
        paths2.ensure_app_dirs().unwrap();

        let result = do_register(&state, &paths2, "alice", "password two");

        let debug = format!("{:?}", result.unwrap_err());
        assert!(debug.contains("username_taken"));
    }

    #[test]
    fn touch_does_not_lock_the_session() {
        let (state, paths, _dir) = test_state_and_paths();
        do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();

        do_touch(&state);

        assert!(!state.session.lock().unwrap().is_locked());
    }

    /// Cross-task self-review finding (Task 13.5): every other test exercises only one hop of the
    /// auth lifecycle in isolation (`commands.rs` tests never call `autolock::tick`; `autolock.rs`
    /// tests never call `do_login`/`do_register`). This test drives the full
    /// register -> idle timeout -> auto-lock -> relogin cycle on one shared `AppState`, proving
    /// the registry connection, vault connection, and session all survive the cycle without
    /// corruption or a stuck state - then repeats the lock/relogin half a second time to catch
    /// anything that only breaks on reuse (e.g. a vault file handle or SQLite lock left over from
    /// the first reopen), not on first use.
    #[test]
    fn full_register_autolock_relogin_lifecycle_survives_on_shared_state() {
        let (state, paths, _dir) = test_state_and_paths();
        let clock = FakeClock::new();
        let timeout = Duration::from_secs(300);

        // 1-2: register, assert unlocked and vault open.
        let reg = do_register(&state, &paths, "alice", "correct horse battery staple").unwrap();
        assert_eq!(reg.username, "alice");
        assert!(state.vault.lock().unwrap().is_some());
        assert!(!state.session.lock().unwrap().is_locked());

        for _ in 0..2 {
            // 3-4: advance the fake clock past timeout, tick, assert just-locked and vault closed.
            // The margin over `timeout` must comfortably exceed the real wall-clock time
            // `do_register`/`do_login` spend on Argon2id (deliberately ~100ms-1s by design, see
            // `core/src/auth.rs`'s timing-oracle-defense comment) between capturing
            // `last_activity` (via the real `SystemClock`) and this `clock.advance` call (on the
            // separate `FakeClock`) - a 1s margin was flaky under that KDF cost; 30s is not.
            clock.advance(timeout + Duration::from_secs(30));
            assert!(crate::autolock::tick(&state, &clock));
            assert!(state.vault.lock().unwrap().is_none());
            assert!(state.session.lock().unwrap().is_locked());

            // 5-6: relogin on the SAME AppState, assert success and vault/session reopened.
            let login =
                do_login(&state, &paths, "alice", "correct horse battery staple").unwrap();
            assert_eq!(login.username, "alice");
            assert!(state.vault.lock().unwrap().is_some());
            assert!(!state.session.lock().unwrap().is_locked());
        }
    }
}
