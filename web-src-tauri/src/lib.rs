//! pwmgr-web — a localhost-only HTTP front end over the shared pwmgr-core command set.
//! Binds 127.0.0.1 exclusively; the port is a fixed constant, never user-configurable.

/// Fixed listen port. Chosen to avoid the Vite dev server (1420) and common dev ports.
pub const PORT: u16 = 7373;
