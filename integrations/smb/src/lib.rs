//! Trusted local bridge. Socket peer identity and the administrator's explicit
//! SMB-user mapping establish authority; request JSON never contains an ACL.
pub mod client;
pub mod protocol;
