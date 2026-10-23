//! This module contains the future directory structure. If possible, new definitions should
//! get added here.
//!
//! Eventually everything should be moved over, and we will move this directory to the top
//! level in `src`.
//!
//! # Basic structure
//!
//! Each child module here represents a library or group of libraries that we are binding. Each of
//! these has several submodules, representing either a directory or a header file in that library.
//!
//! `#include`s turn into `pub use ...*;` statements. Then at the root level (here), we choose
//! which top-level headers we want to reexport the definitions for.
//!
//! All modules are only crate-public since we don't reexport this structure.

mod common;

// Platform libraries: Linux UAPI, including the shared amd64 ABI surface.
cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux_uapi;
        pub(crate) use linux_uapi::*;
    }
}

// Keep alternate Linux amd64 libcs as future, not yet validated product ABIs.
cfg_if! {
    if #[cfg(all(target_os = "linux", target_env = "gnu"))] {
        mod glibc;
        pub(crate) use glibc::*;
    } else if #[cfg(all(target_os = "linux", any(target_env = "musl", target_env = "ohos")))] {
        mod musl;
        pub use musl::sched::*;
        pub(crate) use musl::*;
    } else if #[cfg(all(target_os = "linux", target_env = "uclibc"))] {
        mod uclibc;
        pub(crate) use uclibc::*;
    }
}

// Per-OS headers we export.
cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub use asm::socket::*;
        pub use linux::can::bcm::*;
        pub use linux::can::error::*;
        pub use linux::can::j1939::*;
        pub use linux::can::netlink::*;
        pub use linux::can::raw::*;
        pub use linux::futex::*;
        pub use linux::if_addr::*;
        pub use linux::if_link::*;
        pub use linux::if_packet::*;
        pub use linux::keyctl::*;
        pub use linux::membarrier::*;
        pub use linux::mount::*;
        pub use linux::netlink::*;
        pub use linux::pidfd::*;
        pub use linux::sctp::*;
        pub use linux::tls::*;
        pub use linux::types::*;

        #[cfg(target_env = "gnu")]
        pub use self::{
            net::route::*,
            signal::*,
            sys::socket::*,
            sys::statvfs::*,
        };
        #[cfg(target_env = "uclibc")]
        pub use self::{
            socket::*,
            sysdeps::linux::common::bits::siginfo::*,
        };
    }
}

// Per-env headers we export.
cfg_if! {
    if #[cfg(all(target_os = "linux", any(target_env = "musl", target_env = "ohos")))] {
        pub use signal::*;
        pub use sys::socket::*;
    }
}

// Per-family headers we export.
cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub use pthread::*;
        pub use unistd::*;
    }
}
