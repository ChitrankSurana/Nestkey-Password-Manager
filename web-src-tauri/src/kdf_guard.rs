use pwmgr_core::error::{rate_limited_error, CommandError};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Max concurrent Argon2 derivations (SEC-02/D-12). Each costs 64 MiB, so worst case is ~192 MiB
/// regardless of burst size.
pub const KDF_PERMITS: usize = 3;

/// Shared bounded guard over the Argon2-driving web routes. Rejects immediately when saturated
/// rather than queueing (D-11): a queue is itself a pile-up vector under attack.
#[derive(Clone)]
pub struct KdfGuard(Arc<Semaphore>);

impl KdfGuard {
    pub fn new() -> Self {
        KdfGuard(Arc::new(Semaphore::new(KDF_PERMITS)))
    }

    /// Non-blocking acquire; hold the returned permit for the whole derivation.
    pub fn try_acquire(&self) -> Result<OwnedSemaphorePermit, CommandError> {
        Arc::clone(&self.0)
            .try_acquire_owned()
            .map_err(|_| rate_limited_error("The server is busy. Please try again in a moment."))
    }
}

impl Default for KdfGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permit_boundary_admits_kdf_permits_then_rejects_and_frees_on_drop() {
        let g = KdfGuard::new();
        let mut held: Vec<_> = (0..KDF_PERMITS).map(|_| g.try_acquire().unwrap()).collect();
        assert_eq!(g.try_acquire().unwrap_err().code(), "rate_limited");
        held.pop();
        assert!(g.try_acquire().is_ok());
    }
}
