use crate::session::Clock;
use crate::state::AppState;

/// Returns true only on the transition from unlocked to locked (so the caller emits the
/// `vault-locked` event exactly once per lock, not on every tick of an already-locked session).
pub fn tick(state: &AppState, clock: &dyn Clock) -> bool {
    let mut session = state.session.lock().unwrap();
    if session.is_locked() {
        return false;
    }
    session.check_expiry(clock);
    let just_locked = session.is_locked();
    drop(session);

    if just_locked {
        state.clear_vault();
    }
    just_locked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::paths::AppPaths;
    use rusqlite::Connection;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

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

    fn test_state(timeout: Duration) -> (AppState, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();
        let registry = crate::registry::open_registry(&paths.registry_db_path()).unwrap();
        let state = AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(crate::session::Session::new(timeout)),
            config: Mutex::new(Config::default()),
        };
        (state, dir)
    }

    #[test]
    fn tick_on_already_locked_session_returns_false() {
        let (state, _dir) = test_state(Duration::from_secs(300));
        let clock = FakeClock::new();

        assert!(!tick(&state, &clock));
    }

    #[test]
    fn tick_before_timeout_elapses_stays_unlocked() {
        let (state, _dir) = test_state(Duration::from_secs(300));
        let clock = FakeClock::new();
        let key = crate::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        state.session.lock().unwrap().unlock(key, &clock);

        clock.advance(Duration::from_secs(100));
        assert!(!tick(&state, &clock));
        assert!(!state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn tick_after_timeout_elapses_locks_and_returns_true() {
        let (state, _dir) = test_state(Duration::from_secs(300));
        let clock = FakeClock::new();
        let key = crate::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        state.session.lock().unwrap().unlock(key, &clock);
        state.set_vault(Connection::open_in_memory().unwrap(), PathBuf::from("dummy.vault"));

        clock.advance(Duration::from_secs(301));
        let just_locked = tick(&state, &clock);

        assert!(just_locked);
        assert!(state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_none());
    }
}
