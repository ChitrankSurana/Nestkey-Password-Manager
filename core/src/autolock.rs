use crate::session::Clock;
use crate::state::AppState;
use std::time::Duration;

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

/// True when more real time elapsed between two consecutive scheduled poll-loop ticks than the
/// loop's own sleep interval could explain (with generous slack for scheduler jitter under load) -
/// the system was very likely suspended in between. A suspended process's async timers simply
/// don't fire during suspend, so the gap shows up here as a missed wakeup on resume, rather than
/// through any OS-specific suspend/resume notification API - portable (no platform-specific hook
/// needed) at the cost of only catching actual suspend/hibernate, not a bare screen-lock that
/// leaves the machine running.
pub fn suspend_gap_exceeds(elapsed_since_last_tick: Duration, tick_interval: Duration) -> bool {
    elapsed_since_last_tick > tick_interval * 5
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
    fn suspend_gap_exceeds_is_false_for_a_normal_tick_interval() {
        assert!(!suspend_gap_exceeds(Duration::from_secs(2), Duration::from_secs(2)));
    }

    #[test]
    fn suspend_gap_exceeds_is_false_for_ordinary_scheduler_jitter() {
        assert!(!suspend_gap_exceeds(Duration::from_secs(9), Duration::from_secs(2)));
    }

    #[test]
    fn suspend_gap_exceeds_is_true_after_a_long_gap() {
        assert!(suspend_gap_exceeds(Duration::from_secs(600), Duration::from_secs(2)));
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
        state.session.lock().unwrap().unlock(key, "alice", &clock);

        clock.advance(Duration::from_secs(100));
        assert!(!tick(&state, &clock));
        assert!(!state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn tick_after_timeout_elapses_locks_and_returns_true() {
        let (state, _dir) = test_state(Duration::from_secs(300));
        let clock = FakeClock::new();
        let key = crate::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        state.session.lock().unwrap().unlock(key, "alice", &clock);
        state.set_vault(Connection::open_in_memory().unwrap(), PathBuf::from("dummy.vault"));

        clock.advance(Duration::from_secs(301));
        let just_locked = tick(&state, &clock);

        assert!(just_locked);
        assert!(state.session.lock().unwrap().is_locked());
        assert!(state.vault.lock().unwrap().is_none());
    }
}
