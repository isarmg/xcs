//! Compatibility path for ZXS's shared portable authentication primitives.
//!
//! This platform module confers no storage authority. The single implementation
//! and credential policy live in `zxs-auth`, independent of Linux or SQLite.
pub use zxs_auth::*;
