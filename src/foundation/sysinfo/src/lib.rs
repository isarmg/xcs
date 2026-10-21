// Take a look at the license at the top of the repository in the LICENSE file.

#![cfg_attr(
    all(feature = "system", feature = "disk", feature = "component", feature = "system"),
    doc = include_str!("../../../../docs/upstream/sysinfo-0.39.6/README.md")
)]
#![cfg_attr(
    not(all(
        feature = "system",
        feature = "disk",
        feature = "component",
        feature = "system"
    )),
    doc = "For crate-level documentation, all features need to be enabled."
)]
#![cfg_attr(feature = "serialization", doc = include_str!("../../../../docs/upstream/sysinfo-0.39.6/md_doc/serde.md"))]
#![allow(unknown_lints)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::non_send_fields_in_send_ty)]
#![allow(renamed_and_removed_lints)]
#![allow(clippy::assertions_on_constants)]

// Maintained native target contract; shared Linux ABI implementations stay intact.
#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64", target_pointer_width = "64"),
    all(target_arch = "wasm32", target_os = "unknown", target_pointer_width = "32"),
)))]
compile_error!("foundation sysinfo supports Linux amd64 and wasm32-unknown-unknown fallback only");

#[macro_use]
mod macros;

cfg_select! {
    feature = "unknown-ci" => {
        // This is used in CI to check that the build for unknown targets is compiling fine.
        mod unknown;
        use crate::unknown as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 0;
    }
    target_os = "linux" =>
    {
        mod unix;
        use crate::unix::sys as sys;

        #[cfg(feature = "network")]
        mod network;
        #[cfg(feature = "network")]
        use crate::unix::network_helper;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 1;
    }
    _ => {
        mod unknown;
        use crate::unknown as sys;

        #[cfg(test)]
        pub(crate) const MIN_USERS: usize = 0;
    }
}

#[cfg(feature = "component")]
pub use crate::common::component::{Component, Components};
#[cfg(feature = "disk")]
pub use crate::common::disk::{Disk, DiskKind, DiskRefreshKind, Disks};
#[cfg(feature = "network")]
pub use crate::common::network::{
    InterfaceOperationalState, IpNetwork, IpNetworkFromStrError, MacAddr, MacAddrFromStrError,
    NetworkData, Networks,
};
#[cfg(feature = "system")]
pub use crate::common::system::{
    CGroupLimits, Cpu, CpuRefreshKind, KillError, LoadAvg, MemoryRefreshKind, Motherboard, Pid,
    Process, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, Product, RefreshKind, Signal,
    System, ThreadKind, UpdateKind, get_current_pid,
};
#[cfg(feature = "user")]
pub use crate::common::user::{Group, Groups, User, Users};
#[cfg(any(feature = "user", feature = "system"))]
pub use crate::common::{Gid, Uid};
#[cfg(feature = "system")]
pub use crate::sys::{MINIMUM_CPU_UPDATE_INTERVAL, SUPPORTED_SIGNALS};

#[cfg(any(feature = "system", feature = "disk"))]
pub use crate::common::DiskUsage;

#[cfg(feature = "user")]
pub(crate) use crate::common::user::GroupInner;
#[cfg(feature = "user")]
pub(crate) use crate::sys::UserInner;
#[cfg(feature = "component")]
pub(crate) use crate::sys::{ComponentInner, ComponentsInner};
#[cfg(feature = "system")]
pub(crate) use crate::sys::{CpuInner, MotherboardInner, ProcessInner, ProductInner, SystemInner};
#[cfg(feature = "disk")]
pub(crate) use crate::sys::{DiskInner, DisksInner};
#[cfg(feature = "network")]
pub(crate) use crate::sys::{NetworkDataInner, NetworksInner};

pub use crate::sys::IS_SUPPORTED_SYSTEM;

#[cfg(feature = "c-interface")]
pub use crate::c_interface::*;

#[cfg(feature = "c-interface")]
mod c_interface;
mod common;
mod debug;
#[cfg(feature = "serialization")]
mod data_serialization;
pub(crate) mod utils;

// Make formattable by rustfmt.
#[cfg(any())]
mod network;
#[cfg(any())]
mod unix;
#[cfg(any())]
mod unknown;

/// This function is only used on Linux targets, when the `system` feature is enabled. In other
/// cases, it does nothing and returns `false`.
///
/// On Linux, to improve performance, we keep a `/proc` file open for each process we index with
/// a maximum number of files open equivalent to half of the system limit.
///
/// The problem is that some users might need all the available file descriptors so we need to
/// allow them to change this limit.
///
/// Note that if you set a limit bigger than the system limit, the system limit will be set.
///
/// Returns `true` if the new value has been set.
///
#[cfg_attr(feature = "system", doc = "```no_run")]
#[cfg_attr(not(feature = "system"), doc = "```ignore")]
/// use sysinfo::{System, set_open_files_limit};
///
/// // We call the function before any call to the processes update.
/// if !set_open_files_limit(10) {
///     // It'll always return false on non-linux targets.
///     eprintln!("failed to update the open files limit...");
/// }
/// let s = System::new_all();
/// ```
pub fn set_open_files_limit(mut _new_limit: usize) -> bool {
    cfg_select! {
        all(feature = "system", not(feature = "unknown-ci"), any(target_os = "linux", target_os = "android")) =>
        {
            use crate::sys::system::remaining_files;
            use std::sync::atomic::Ordering;

            let max = sys::system::get_max_nb_fds();
            if _new_limit > max {
                _new_limit = max;
            }

            // If files are already open, to be sure that the number won't be bigger when those
            // files are closed, we subtract the current number of opened files to the new
            // limit.
            if remaining_files().try_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                let _new_limit = _new_limit as isize;
                let diff = (max as isize).saturating_sub(remaining);
                Some(_new_limit.saturating_sub(diff))
            }).is_err() {
                sysinfo_debug!("failed to update open files limit");
            }

            true
        }
        _ => {
            false
        }
    }
}

#[cfg(doctest)]
mod doctest {
    macro_rules! compile_fail_import {
        ($mod_name:ident => $($imports:ident),+ $(,)?) => {
            $(#[doc = concat!(r"```compile_fail
use sysinfo::", stringify!($imports), r";
```
")])+
            mod $mod_name {}
        };
    }

    #[cfg(not(feature = "system"))]
    compile_fail_import!(
        no_system_feature =>
        get_current_pid,
        CGroupLimits,
        Cpu,
        CpuRefreshKind,
        DiskUsage,
        KillError,
        LoadAvg,
        MemoryRefreshKind,
        Motherboard,
        Pid,
        Process,
        ProcessesToUpdate,
        ProcessRefreshKind,
        ProcessStatus,
        Product,
        RefreshKind,
        Signal,
        System,
        ThreadKind,
        UpdateKind,
    );

    #[cfg(not(feature = "disk"))]
    compile_fail_import!(
        no_disk_feature =>
        Disk,
        Disks,
        DiskKind,
    );

    #[cfg(not(feature = "component"))]
    compile_fail_import!(
        no_component_feature =>
        Component,
        Components,
    );

    #[cfg(not(feature = "network"))]
    compile_fail_import!(
        no_network_feature =>
        IpNetwork,
        MacAddr,
        NetworkData,
        Networks,
    );

    #[cfg(not(feature = "user"))]
    compile_fail_import!(
        no_user_feature =>
        Group,
        Groups,
        User,
        Users,
    );
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/sysinfo/unit/src/lib/test_251.rs"));
