//! Compatibility path for XCS's shared portable authentication primitives.
//!
//! This platform module confers no storage authority. The single implementation
//! and credential policy live in `xcs-auth`, independent of Linux or SQLite.
pub use xcs_auth::*;
