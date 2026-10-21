#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
/*
Unsafe functions no longer implicitly create unsafe blocks (RFC 2585).
While newer bindgen versions follow this convention, we support Rust 1.63 which warns about
this behavior. We can simply silence these warnings.
*/
#![allow(unused_unsafe)]

// Local platform scope. Original generated bindings remain in provenance receipts.
#[cfg(not(all(
    target_os = "linux",
    target_arch = "x86_64",
    target_pointer_width = "64"
)))]
compile_error!("the maintained linux-raw-sys bindings require Linux amd64 (LP64)");

#[cfg(feature = "std")]
pub use std::os::raw as ctypes;

#[cfg(all(not(feature = "std"), feature = "no_std"))]
pub mod ctypes {
    // The maintained Linux amd64 ABI uses signed char.
    pub type c_char = c_schar;

    // The maintained Linux amd64 ABI is LP64, and char is always 8-bit.
    //
    // In theory, `c_long` and `c_ulong` could be `isize` and `usize`
    // respectively, however in practice Linux doesn't use them in that way
    // consistently. So stick with the convention followed by `libc` and
    // others and use the fixed-width types.
    pub type c_schar = i8;
    pub type c_uchar = u8;
    pub type c_short = i16;
    pub type c_ushort = u16;
    pub type c_int = i32;
    pub type c_uint = u32;
    pub type c_long = i64;
    pub type c_ulong = u64;
    pub type c_longlong = i64;
    pub type c_ulonglong = u64;
    pub type c_float = f32;
    pub type c_double = f64;

    pub use core::ffi::c_void;
}

// Confirm the retained ABI without restoring unrelated upstream dev dependencies.
#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/linux-raw-sys/unit/src/lib/assertions_49.rs"));

// We don't enable `derive_eq` in bindgen because adding `PartialEq`/`Eq` to
// *all* structs noticeably increases compile times. But we can add a few
// manual impls where they're especially useful.
#[cfg(feature = "general")]
impl PartialEq for general::__kernel_timespec {
    fn eq(&self, other: &Self) -> bool {
        ({
            let Self { tv_sec, tv_nsec } = self;
            (tv_sec, tv_nsec)
        }) == ({
            let Self { tv_sec, tv_nsec } = other;
            (tv_sec, tv_nsec)
        })
    }
}
#[cfg(feature = "general")]
impl Eq for general::__kernel_timespec {}

#[cfg(feature = "net")]
pub mod cmsg_macros {
    use crate::ctypes::{c_long, c_uchar, c_uint};
    use crate::net::{cmsghdr, msghdr};
    use core::mem::size_of;
    use core::ptr;

    pub const unsafe fn CMSG_ALIGN(len: c_uint) -> c_uint {
        let c_long_size = size_of::<c_long>() as c_uint;
        (len + c_long_size - 1) & !(c_long_size - 1)
    }

    pub const unsafe fn CMSG_DATA(cmsg: *const cmsghdr) -> *mut c_uchar {
        (cmsg as *mut c_uchar).add(size_of::<cmsghdr>())
    }

    pub const unsafe fn CMSG_SPACE(len: c_uint) -> c_uint {
        size_of::<cmsghdr>() as c_uint + CMSG_ALIGN(len)
    }

    pub const unsafe fn CMSG_LEN(len: c_uint) -> c_uint {
        size_of::<cmsghdr>() as c_uint + len
    }

    pub const unsafe fn CMSG_FIRSTHDR(mhdr: *const msghdr) -> *mut cmsghdr {
        if (*mhdr).msg_controllen < size_of::<cmsghdr>() as _ {
            return ptr::null_mut();
        }

        (*mhdr).msg_control as *mut cmsghdr
    }

    pub unsafe fn CMSG_NXTHDR(mhdr: *const msghdr, cmsg: *const cmsghdr) -> *mut cmsghdr {
        // We convert from raw pointers to usize here, which may not be sound in a
        // future version of Rust. Once the provenance rules are set in stone,
        // it will be a good idea to give this function a once-over.

        let cmsg_len = (*cmsg).cmsg_len;
        let next_cmsg = (cmsg as *mut u8).add(CMSG_ALIGN(cmsg_len as _) as usize) as *mut cmsghdr;
        let max = ((*mhdr).msg_control as usize) + ((*mhdr).msg_controllen as usize);

        if cmsg_len < size_of::<cmsghdr>() as _ {
            return ptr::null_mut();
        }

        if next_cmsg.add(1) as usize > max
            || next_cmsg as usize + CMSG_ALIGN((*next_cmsg).cmsg_len as _) as usize > max
        {
            return ptr::null_mut();
        }

        next_cmsg
    }
}

#[cfg(feature = "general")]
pub mod select_macros {
    use crate::ctypes::c_int;
    use crate::general::__kernel_fd_set;
    use core::mem::size_of;

    pub unsafe fn FD_CLR(fd: c_int, set: *mut __kernel_fd_set) {
        let bytes = set as *mut u8;
        if fd >= 0 {
            *bytes.add((fd / 8) as usize) &= !(1 << (fd % 8));
        }
    }

    pub unsafe fn FD_SET(fd: c_int, set: *mut __kernel_fd_set) {
        let bytes = set as *mut u8;
        if fd >= 0 {
            *bytes.add((fd / 8) as usize) |= 1 << (fd % 8);
        }
    }

    pub unsafe fn FD_ISSET(fd: c_int, set: *const __kernel_fd_set) -> bool {
        let bytes = set as *const u8;
        if fd >= 0 {
            *bytes.add((fd / 8) as usize) & (1 << (fd % 8)) != 0
        } else {
            false
        }
    }

    pub unsafe fn FD_ZERO(set: *mut __kernel_fd_set) {
        let bytes = set as *mut u8;
        core::ptr::write_bytes(bytes, 0, size_of::<__kernel_fd_set>());
    }
}

#[cfg(feature = "general")]
pub mod signal_macros {
    pub const SIG_DFL: super::general::__kernel_sighandler_t = None;

    /// Rust doesn't currently permit us to use `transmute` to convert the
    /// `SIG_IGN` value into a function pointer in a `const` initializer, so
    /// we make it a function instead.
    ///
    #[inline]
    pub const fn sig_ign() -> super::general::__kernel_sighandler_t {
        // Safety: This creates an invalid pointer, but the pointer type
        // includes `unsafe`, which covers the safety of calling it.
        Some(unsafe {
            core::mem::transmute::<usize, unsafe extern "C" fn(crate::ctypes::c_int)>(1)
        })
    }
}

#[cfg(feature = "elf")]
pub mod elf;

// Retained generated bindings: every Linux x86_64 API feature.
#[cfg(feature = "auxvec")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/auxvec.rs"]
pub mod auxvec;
#[cfg(feature = "bootparam")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/bootparam.rs"]
pub mod bootparam;
#[cfg(feature = "btrfs")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/btrfs.rs"]
pub mod btrfs;
#[cfg(feature = "elf_uapi")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/elf_uapi.rs"]
pub mod elf_uapi;
#[cfg(feature = "errno")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/errno.rs"]
pub mod errno;
#[cfg(feature = "general")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/general.rs"]
pub mod general;
#[cfg(feature = "if_arp")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/if_arp.rs"]
pub mod if_arp;
#[cfg(feature = "if_ether")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/if_ether.rs"]
pub mod if_ether;
#[cfg(feature = "if_packet")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/if_packet.rs"]
pub mod if_packet;
#[cfg(feature = "if_tun")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/if_tun.rs"]
pub mod if_tun;
#[cfg(feature = "image")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/image.rs"]
pub mod image;
#[cfg(feature = "io_uring")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/io_uring.rs"]
pub mod io_uring;
#[cfg(feature = "ioctl")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/ioctl.rs"]
pub mod ioctl;
#[cfg(feature = "landlock")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/landlock.rs"]
pub mod landlock;
#[cfg(feature = "loop_device")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/loop_device.rs"]
pub mod loop_device;
#[cfg(feature = "mempolicy")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/mempolicy.rs"]
pub mod mempolicy;
#[cfg(feature = "net")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/net.rs"]
pub mod net;
#[cfg(feature = "netlink")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/netlink.rs"]
pub mod netlink;
#[cfg(feature = "prctl")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/prctl.rs"]
pub mod prctl;
#[cfg(feature = "ptrace")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/ptrace.rs"]
pub mod ptrace;
#[cfg(feature = "system")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/system.rs"]
pub mod system;
#[cfg(feature = "vm_sockets")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/vm_sockets.rs"]
pub mod vm_sockets;
#[cfg(feature = "xdp")]
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
#[path = "x86_64/xdp.rs"]
pub mod xdp;
