//! ZXS's independent, narrowly selected platform foundation.
//!
//! SQLite is a rebuildable query index. Neither its transactions nor any
//! authentication/session helper constitute a cross-disk commit decision.
//! The storage engine owns ACCEPT, FINALIZE, PUBLICATION, and recovery.
#![deny(unsafe_code, unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "linux"))]
compile_error!(
    "zxs-platform is the Linux server foundation; portable clients must use zxs-contract"
);

pub mod error;
pub mod fs_safety;
pub mod payload;
pub mod schema_identity;
pub mod secret;
pub mod sqlite;
