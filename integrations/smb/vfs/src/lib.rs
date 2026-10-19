//! Rust SMB VFS extension. Building the loadable module requires a configured
//! target 4.23.0 SDK, selected explicitly with the smb-sdk feature.
#[cfg(not(feature = "smb-sdk"))]
compile_error!(
    "the loadable XCS VFS module requires smb-sdk; build xcs-smb for protocol-only tools"
);
#[cfg(feature = "smb-sdk")]
mod module;
