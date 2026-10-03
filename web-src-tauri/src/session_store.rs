use nestkey_core::autolock;
use nestkey_core::session::Clock;
use nestkey_core::state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

/// Opaque session token (256 bits, hex-encoded → 64 chars).
pub type Token = String;

/// A locked-but-retained entry is pruned once it has been idle this long, bounding memory growth.
pub const PRUNE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// A single in-memory upload slot cached on a `UserSession` (SEC-01/D-04): the browser uploads a
/// CSV or backup file body once, and every downstream step (headers/preview/commit for CSV, or
/// restore for a backup) reads from this cache instead of a client-supplied filesystem path. Bytes
/// are held in `Zeroizing<Vec<u8>>` so the plaintext credentials an uploaded file may contain are
/// wiped from memory the moment the cache is cleared or replaced.
pub enum UploadCache {
    Csv(Zeroizing<Vec<u8>>),
    Backup(Zeroizing<Vec<u8>>),
    /// The full-fidelity JSON vault export/import feature's upload slot - same one-shot cache
    /// discipline as Csv/Backup above.
    Json(Zeroizing<Vec<u8>>),
}

/// One logged-in browser session. Owns a full `AppState` (its own vault connection + lock timer +
/// registry connection to the shared registry.db — see the plan's Finding 4) plus a `last_seen`
/// timestamp, bumped on every authenticated request, used only for the 24h prune.
pub struct UserSession {
    pub username: String,
    pub app_state: AppState,
    pub last_seen: Mutex<Instant>,
    /// The session's single cached file upload, if any — see `UploadCache`.
    pub upload_cache: Mutex<Option<UploadCache>>,
}

impl UserSession {
    pub fn bump_last_seen(&self, clock: &dyn Clock) {
        *self.last_seen.lock().unwrap() = clock.now();
    }

    /// Replaces the single upload slot under one critical section on `upload_cache`'s own mutex,
    /// holding no other lock while doing so. A second upload arriving mid-flight (e.g. two
    /// concurrent `/api/upload_csv` requests on the same session) therefore always replaces the
    /// slot atomically rather than appending to or interleaving with the previous buffer — a
    /// subsequent read always observes exactly one complete upload, never a spliced/partial one.
    /// There is deliberately no "append to the existing buffer" API.
    pub fn set_upload(&self, cache: UploadCache) {
        *self.upload_cache.lock().unwrap() = Some(cache);
    }

    /// Clones out the cached CSV bytes, if any. Returns `None` if nothing is cached or the cached
    /// upload is a `Backup`, not a `Csv`.
    pub fn csv_upload(&self) -> Option<Vec<u8>> {
        match &*self.upload_cache.lock().unwrap() {
            Some(UploadCache::Csv(bytes)) => Some(bytes.to_vec()),
            _ => None,
        }
    }

    /// Clones out the cached backup bytes, if any. Returns `None` if nothing is cached or the
    /// cached upload is a `Csv`/`Json`, not a `Backup`.
    pub fn backup_upload(&self) -> Option<Vec<u8>> {
        match &*self.upload_cache.lock().unwrap() {
            Some(UploadCache::Backup(bytes)) => Some(bytes.to_vec()),
            _ => None,
        }
    }

    /// Clones out the cached JSON vault export bytes, if any.
    pub fn json_upload(&self) -> Option<Vec<u8>> {
        match &*self.upload_cache.lock().unwrap() {
            Some(UploadCache::Json(bytes)) => Some(bytes.to_vec()),
            _ => None,
        }
    }

    /// Drops the cached upload, if any — the `Zeroizing` wrapper wipes the bytes on drop.
    pub fn clear_upload(&self) {
        *self.upload_cache.lock().unwrap() = None;
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

    /// Forces every live session locked immediately - used when the host process itself appears to
    /// have been suspended (`autolock::suspend_gap_exceeds` on the sweep loop's own tick interval),
    /// rather than waiting for each session's own idle timeout to eventually catch up.
    pub fn lock_all(&self) {
        let map = self.inner.read().unwrap();
        for session in map.values() {
            nestkey_core::commands::do_lock(&session.app_state);
        }
    }
}

/// 256-bit token from the OS CSPRNG (`rand::rngs::OsRng`), hex-encoded — same secure-randomness
/// source nestkey-core already uses for salts/nonces (`core/src/crypto.rs`).
fn new_token() -> Token {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nestkey_core::config::Config;
    use nestkey_core::paths::AppPaths;
    use nestkey_core::session::{Clock, Session};
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
        let registry = nestkey_core::registry::open_registry(&paths.registry_db_path()).unwrap();
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
            upload_cache: Mutex::new(None),
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
        let key = nestkey_core::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        us.app_state.session.lock().unwrap().unlock(key, "alice", &clock);
        let token = store.insert(us);
        assert!(!store.get(&token).unwrap().app_state.session.lock().unwrap().is_locked());

        clock.advance(Duration::from_secs(301));
        store.sweep(&clock);

        let s = store.get(&token).expect("entry must persist after auto-lock");
        assert!(s.app_state.session.lock().unwrap().is_locked());
    }

    #[test]
    fn lock_all_locks_every_unlocked_session_immediately() {
        let clock = FakeClock::new();
        let store = SessionStore::new();
        let (a, _da) = test_session(&clock, Duration::from_secs(300));
        let (b, _db) = test_session(&clock, Duration::from_secs(300));
        let key = nestkey_core::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        a.app_state.session.lock().unwrap().unlock(key, "alice", &clock);
        let ta = store.insert(a);
        let tb = store.insert(b);

        store.lock_all();

        assert!(store.get(&ta).unwrap().app_state.session.lock().unwrap().is_locked());
        assert!(store.get(&tb).unwrap().app_state.session.lock().unwrap().is_locked());
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
        let key = nestkey_core::crypto::derive_key("pw", &[1u8; 16]).unwrap();
        a.app_state.session.lock().unwrap().unlock(key, "alice", &clock);
        let ta = store.insert(a);
        let tb = store.insert(b);

        assert!(!store.get(&ta).unwrap().app_state.session.lock().unwrap().is_locked());
        assert!(store.get(&tb).unwrap().app_state.session.lock().unwrap().is_locked());
    }
}
