//! Trusted local bridge. Socket peer identity and the configured authenticated
//! Samba account bind the sole current ZXS account; request JSON never carries ACLs.
pub mod client;
pub mod protocol;
