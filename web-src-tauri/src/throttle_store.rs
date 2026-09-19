use pwmgr_core::session::Clock;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Attempts admitted on a key before it is locked out (D-08).
pub const FAIL_THRESHOLD: u32 = 5;
/// First lockout window; each later cycle doubles it up to `MAX_WINDOW` (D-08).
pub const BASE_WINDOW: Duration = Duration::from_secs(30);
/// Cap on the doubling lockout window, so a legitimate user is never locked out for long.
pub const MAX_WINDOW: Duration = Duration::from_secs(60 * 60);
/// An unlocked entry idle this long is pruned by `sweep`, bounding memory growth.
pub const THROTTLE_PRUNE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

struct ThrottleEntry {
    fail_count: u32,
    /// Length of the lockout the NEXT time `fail_count` reaches the threshold.
    current_window: Duration,
    locked_until: Option<Instant>,
    last_seen: Instant,
}

/// In-process per-key attempt throttle (SEC-02, D-06): never persisted, so a restart clears it.
#[derive(Clone, Default)]
pub struct ThrottleStore {
    inner: Arc<RwLock<HashMap<String, ThrottleEntry>>>,
}

impl ThrottleStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decides whether an attempt on `key` may proceed AND counts it, in one write-lock critical
    /// section. Counting at attempt start (not on failure) means N parallel attempts are linearized:
    /// the sixth is refused even while the first five are still in flight, so a concurrent burst
    /// cannot exceed the threshold. `Err(remaining)` means the key is locked out.
    pub fn begin_attempt(&self, key: &str, clock: &dyn Clock) -> Result<(), Duration> {
        let now = clock.now();
        let mut map = self.inner.write().unwrap();
        let e = map.entry(key.to_string()).or_insert(ThrottleEntry {
            fail_count: 0,
            current_window: BASE_WINDOW,
            locked_until: None,
            last_seen: now,
        });
        e.last_seen = now;
        if let Some(until) = e.locked_until {
            if now < until {
                return Err(until - now);
            }
            e.locked_until = None;
            e.fail_count = 0;
        }
        e.fail_count += 1;
        if e.fail_count >= FAIL_THRESHOLD {
            e.locked_until = Some(now + e.current_window);
            e.fail_count = 0;
            // saturating_mul: a Duration overflow panic under the write lock would poison the map.
            e.current_window = e.current_window.saturating_mul(2).min(MAX_WINDOW);
        }
        Ok(())
    }

    /// Successful authentication: forget the key entirely (counter, window multiplier, lock) - D-10.
    pub fn record_success(&self, key: &str) {
        self.inner.write().unwrap().remove(key);
    }

    /// Drops entries that are neither locked nor seen within `THROTTLE_PRUNE_AFTER`.
    pub fn sweep(&self, clock: &dyn Clock) {
        let now = clock.now();
        self.inner.write().unwrap().retain(|_, e| {
            e.locked_until.is_some_and(|u| now < u)
                || now.duration_since(e.last_seen) < THROTTLE_PRUNE_AFTER
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

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

    /// Burns the five admitted attempts, then returns the lockout remaining on the sixth.
    fn lock_out(store: &ThrottleStore, key: &str, clock: &FakeClock) -> Duration {
        for _ in 0..FAIL_THRESHOLD {
            store.begin_attempt(key, clock).unwrap();
        }
        store.begin_attempt(key, clock).unwrap_err()
    }

    #[test]
    fn five_then_locked() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        assert_eq!(lock_out(&s, "k", &c), BASE_WINDOW);
    }

    #[test]
    fn unlocks_after_the_window_elapses() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        lock_out(&s, "k", &c);
        c.advance(BASE_WINDOW);
        assert!(s.begin_attempt("k", &c).is_ok());
    }

    #[test]
    fn windows_double_across_cycles() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        let w1 = lock_out(&s, "k", &c);
        c.advance(w1);
        let w2 = lock_out(&s, "k", &c);
        c.advance(w2);
        let w3 = lock_out(&s, "k", &c);
        assert_eq!((w1, w2, w3), (Duration::from_secs(30), Duration::from_secs(60), Duration::from_secs(120)));
    }

    #[test]
    fn window_is_capped_at_one_hour() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        let mut w = Duration::ZERO;
        for _ in 0..20 {
            w = lock_out(&s, "k", &c);
            assert!(w <= MAX_WINDOW);
            c.advance(w);
        }
        assert_eq!(w, MAX_WINDOW);
    }

    #[test]
    fn record_success_restores_a_fresh_key() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        let w1 = lock_out(&s, "k", &c);
        c.advance(w1);
        lock_out(&s, "k", &c); // second cycle: window now doubled
        s.record_success("k");
        assert_eq!(lock_out(&s, "k", &c), BASE_WINDOW);
    }

    #[test]
    fn keys_do_not_interfere() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        lock_out(&s, "a", &c);
        assert!(s.begin_attempt("b", &c).is_ok());
    }

    #[test]
    fn sweep_prunes_stale_unlocked_entries_only() {
        let (s, c) = (ThrottleStore::new(), FakeClock::new());
        s.begin_attempt("idle", &c).unwrap();
        c.advance(THROTTLE_PRUNE_AFTER + Duration::from_secs(1));
        s.sweep(&c);
        assert!(s.inner.read().unwrap().is_empty());
    }
}
