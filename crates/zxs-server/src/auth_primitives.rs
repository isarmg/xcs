//! Compatibility path for ZXS's shared portable authentication primitives.
//!
//! Algorithms and their regression tests live in `zxs-auth`; HTTP admission,
//! account persistence and session expiry remain owned by this server.
pub use zxs_auth::*;
