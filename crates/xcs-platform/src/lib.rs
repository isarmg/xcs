//! XCS's independent, narrowly selected platform foundation.
//!
//! SQLite is a rebuildable query index. Neither its transactions nor any
//! authentication/session helper constitute a cross-disk commit decision.
//! The storage engine owns ACCEPT, FINALIZE, PUBLICATION, and recovery.
#![deny(unsafe_code, unsafe_op_in_unsafe_fn)]

#[cfg(not(target_os = "linux"))]
compile_error!(
    "xcs-platform is the Linux server foundation; portable clients must use xcs-contract"
);

pub mod admin_auth;
pub mod error;
pub mod fs_safety;
pub mod payload;
pub mod schema_identity;
pub mod secret;
pub mod sqlite;
