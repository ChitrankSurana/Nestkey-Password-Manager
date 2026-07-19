use pwmgr_core::autolock;
use pwmgr_core::session::Clock;
use pwmgr_core::state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Opaque session token (256 bits, hex-encoded → 64 chars).
pub type Token = String;

/// A locked-but-retained entry is pruned once it has been idle this long, bounding memory growth.
pub const PRUNE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// One logged-in browser session. Owns a full `AppState` (its own vault connection + lock timer +
/// registry connection to the shared registry.db — see the plan's Finding 4) plus a `last_seen`
/// timestamp, bumped on every authenticated request, used only for the 24h prune.
pub struct UserSession {
    pub username: String,
    pub app_state: AppState,
    pub last_seen: Mutex<Instant>,
}

impl UserSession {
    pub fn bump_last_seen(&self, clock: &dyn Clock) {
        *self.last_seen.lock().unwrap() = clock.now();
    }
}

/// Thread-safe `token → UserSession` map shared across all axum handlers and the sweep task.
#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<Token, Arc<UserSession>>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a session under a fresh random token and returns it.
    pub fn insert(&self, session: UserSession) -> Token {
        let token = new_token();
        self.inner.write().unwrap().insert(token.clone(), Arc::new(session));
        token
    }

    pub fn get(&self, token: &str) -> Option<Arc<UserSession>> {
        self.inner.read().unwrap().get(token).cloned()
    }

    pub fn remove(&self, token: &str) {
        self.inner.write().unwrap().remove(token);
    }

    /// Ticks every entry's idle auto-lock (locking + clearing that entry's vault on timeout, exactly
    /// like the single-session desktop autolock — this is the map-iteration generalization the spec
    /// flagged), then prunes any entry untouched for `PRUNE_AFTER`. Never holds the map write lock
    /// while ticking (ticking takes the read lock), so a slow handler can't be starved by the sweep.
    pub fn sweep(&self, clock: &dyn Clock) {
        {
            let map = self.inner.read().unwrap();
            for session in map.values() {
                autolock::tick(&session.app_state, clock);
            }
        }
        let now = clock.now();
        self.inner
            .write()
            .unwrap()
            .retain(|_, s| now.duration_since(*s.last_seen.lock().unwrap()) < PRUNE_AFTER);
    }
}

/// 256-bit token from the OS CSPRNG (`rand::rngs::OsRng`), hex-encoded — same secure-randomness
/// source pwmgr-core already uses for salts/nonces (`core/src/crypto.rs`).
fn new_token() -> Token {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwmgr_core::config::Config;
    use pwmgr_core::paths::AppPaths;
    use pwmgr_core::session::{Clock, Session};
    use std::cell::RefCell;
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

    fn test_session(clock: &dyn Clock, timeout: Duration) -> (UserSession, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf());
        paths.ensure_app_dirs().unwrap();
        let registry = pwmgr_core::registry::open_registry(&paths.registry_db_path()).unwrap();
        let app_state = AppState {
            registry: Mutex::new(registry),
            vault: Mutex::new(None),
            session: Mutex::new(Session::new(timeout)),
            config: Mutex::new(Config::default()),
        };
        let us = UserSession {
            username: "alice".into(),
            app_state,
            last_seen: Mutex::new(clock.now()),
        };
        (us, dir)
    }

    #[test]
    fn insert_returns_unique_high_entropy_tokens() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (a, _da) = test_session(&clock, Duration::from_secs(300));
        let (b, _db) = test_session(&clock, Duration::from_secs(300));
        let ta = store.insert(a);
        let tb = store.insert(b);
        assert_ne!(ta, tb);
        assert_eq!(ta.len(), 64); // 32 bytes, hex-encoded
        assert!(ta.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn insert_then_get_returns_the_session() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (us, _d) = test_session(&clock, Duration::from_secs(300));
        let token = store.insert(us);
        assert_eq!(store.get(&token).unwrap().username, "alice");
    }

    #[test]
    fn get_unknown_token_returns_none() {
        let store = SessionStore::new();
        assert!(store.get("deadbeef").is_none());
    }

    #[test]
    fn remove_deletes_the_entry() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (us, _d) = test_session(&clock, Duration::from_secs(300));
        let token = store.insert(us);
        store.remove(&token);
        assert!(store.get(&token).is_none());
    }

    #[test]
    fn sweep_locks_an_idle_unlocked_entry_but_keeps_it_in_the_map() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (us, _d) = test_session(&clock, Duration::from_secs(300));
        let key = pwmgr_core::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        us.app_state.session.lock().unwrap().unlock(key, &clock);
        let token = store.insert(us);
        assert!(!store.get(&token).unwrap().app_state.session.lock().unwrap().is_locked());

        clock.advance(Duration::from_secs(301));
        store.sweep(&clock);

        let s = store.get(&token).expect("entry must persist after auto-lock");
        assert!(s.app_state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn sweep_prunes_an_entry_after_24h_of_inactivity() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (us, _d) = test_session(&clock, Duration::from_secs(300));
        let token = store.insert(us);

        clock.advance(PRUNE_AFTER + Duration::from_secs(1));
        store.sweep(&clock);

        assert!(store.get(&token).is_none(), "entry idle > 24h must be pruned");
    }

    #[test]
    fn two_sessions_keep_independent_lock_state() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (a, _da) = test_session(&clock, Duration::from_secs(300));
        let (b, _db) = test_session(&clock, Duration::from_secs(300));
        let key = pwmgr_core::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        a.app_state.session.lock().unwrap().unlock(key, &clock);
        let ta = store.insert(a);
        let tb = store.insert(b);

        assert!(!store.get(&ta).unwrap().app_state.session.lock().unwrap().is_locked());
        assert!(store.get(&tb).unwrap().app_state.session.lock().unwrap().is_locked());
    }
}
