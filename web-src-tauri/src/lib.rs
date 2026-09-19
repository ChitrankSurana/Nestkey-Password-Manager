//! pwmgr-web — a localhost-only HTTP front end over the shared pwmgr-core command set.
//! Binds 127.0.0.1 exclusively; the port is a fixed constant, never user-configurable.

pub mod error_map;
pub mod handlers;
pub mod middleware;
pub mod session_store;
pub mod throttle_store;

pub use handlers::{build_router, AppContext};
pub use session_store::SessionStore;
pub use throttle_store::ThrottleStore;

/// Fixed listen port. Chosen to avoid the Vite dev server (1420) and common dev ports.
pub const PORT: u16 = 7373;
