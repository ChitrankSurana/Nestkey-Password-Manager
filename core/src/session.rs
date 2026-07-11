use crate::crypto::VaultKey;
use std::cell::RefCell;
use std::time::{Duration, Instant};

pub trait Clock {
    fn now(&self) -> Instant;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

pub struct Session {
    key: Option<VaultKey>,
    last_activity: Option<Instant>,
    timeout: Duration,
}

impl Session {
    pub fn new(timeout: Duration) -> Self {
        Session { key: None, last_activity: None, timeout }
    }

    pub fn unlock(&mut self, key: VaultKey, clock: &dyn Clock) {
        self.key = Some(key);
        self.last_activity = Some(clock.now());
    }

    pub fn touch(&mut self, clock: &dyn Clock) {
        if self.key.is_some() {
            self.last_activity = Some(clock.now());
        }
    }

    pub fn lock(&mut self) {
        self.key = None; // VaultKey is ZeroizeOnDrop — dropping it here zeroizes the bytes.
        self.last_activity = None;
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn is_locked(&self) -> bool {
        self.key.is_none()
    }

    pub fn check_expiry(&mut self, clock: &dyn Clock) {
        if let Some(last) = self.last_activity {
            if clock.now().duration_since(last) >= self.timeout {
                self.lock();
            }
        }
    }

    pub fn key(&self) -> Option<&VaultKey> {
        self.key.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::derive_key;

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
    fn new_session_starts_locked() {
        let session = Session::new(Duration::from_secs(300));
        assert!(session.is_locked());
        assert!(session.key().is_none());
    }

    #[test]
    fn unlock_makes_session_unlocked_and_exposes_key() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        let key = derive_key("pw", &[1u8; 16]).unwrap();

        session.unlock(key, &clock);

        assert!(!session.is_locked());
        assert!(session.key().is_some());
    }

    #[test]
    fn lock_clears_the_key() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        session.lock();

        assert!(session.is_locked());
        assert!(session.key().is_none());
    }

    #[test]
    fn check_expiry_locks_after_timeout_elapses_with_no_activity() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        clock.advance(Duration::from_secs(301));
        session.check_expiry(&clock);

        assert!(session.is_locked());
    }

    #[test]
    fn touch_resets_the_idle_timer_preventing_expiry() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        clock.advance(Duration::from_secs(200));
        session.touch(&clock);
        clock.advance(Duration::from_secs(200)); // 400s since unlock, but only 200s since touch
        session.check_expiry(&clock);

        assert!(!session.is_locked());
    }

    #[test]
    fn check_expiry_on_already_locked_session_is_a_no_op() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.check_expiry(&clock); // locked session, nothing to expire
        assert!(session.is_locked());
    }

    #[test]
    fn set_timeout_changes_the_idle_timeout_on_a_live_session() {
        let clock = FakeClock::new();
        let mut session = Session::new(Duration::from_secs(300));
        session.unlock(derive_key("pw", &[1u8; 16]).unwrap(), &clock);

        session.set_timeout(Duration::from_secs(60));
        clock.advance(Duration::from_secs(61));
        session.check_expiry(&clock);

        assert!(session.is_locked(), "shortened timeout should have expired the session");
    }

    #[test]
    fn set_timeout_on_a_locked_session_does_not_unlock_it() {
        let mut session = Session::new(Duration::from_secs(300));
        session.set_timeout(Duration::from_secs(60));
        assert!(session.is_locked());
    }
}
