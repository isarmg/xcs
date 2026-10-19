//! Compatibility path for XCS's shared portable authentication primitives.
//!
//! Algorithms and their regression tests live in `xcs-auth`; HTTP admission,
//! account persistence and session expiry remain owned by this server.
pub use xcs_auth::*;
