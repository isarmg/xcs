//! Architecture-specific syscall code.
//!
//! This module also has a `choose` submodule which chooses a scheme and is
//! what most of the `rustix` syscalls use.
//!
//! Compilers should really have intrinsics for making system calls. They're
//! much like regular calls, with custom calling conventions, and calling
//! conventions are otherwise the compiler's job. But for now, use inline asm.
//!
//! The calling conventions for Linux syscalls are [documented here].
//!
//! [documented here]: https://man7.org/linux/man-pages/man2/syscall.2.html
//!
//! # Safety
//!
//! This contains the inline `asm` statements performing the syscall
//! instructions.

#![allow(unsafe_code)]
#![cfg_attr(not(feature = "all-apis"), allow(unused_imports))]
// We'll use as many arguments as syscalls need.
#![allow(clippy::too_many_arguments)]

// Linux amd64 uses the syscall instruction directly. Keep the implementation
// byte-identical to the upstream source; target rejection lives in build.rs.
#[path = "x86_64.rs"]
pub(in crate::backend) mod asm;
pub(in crate::backend) use self::asm as choose;

// Macros for invoking system calls.
//
// These factor out:
//  - Calling `nr` on the syscall number to convert it into `SyscallNumber`.
//  - Calling `.into()` on each of the arguments to convert them into `ArgReg`.
//  - Qualifying the `syscall*` and `__NR_*` identifiers.
//  - Counting the number of arguments.
macro_rules! syscall {
    ($nr:ident) => {
        $crate::backend::arch::choose::syscall0($crate::backend::reg::nr(
            linux_raw_sys::general::$nr,
        ))
    };

    ($nr:ident, $a0:expr) => {
        $crate::backend::arch::choose::syscall1(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr) => {
        $crate::backend::arch::choose::syscall2(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr) => {
        $crate::backend::arch::choose::syscall3(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {
        $crate::backend::arch::choose::syscall4(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
        $crate::backend::arch::choose::syscall5(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
        $crate::backend::arch::choose::syscall6(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
            $a5.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
        $crate::backend::arch::choose::syscall7(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
            $a5.into(),
            $a6.into(),
        )
    };
}

// Macro to invoke a syscall that always uses direct assembly, rather than the
// vDSO. Useful when still finding the vDSO.
#[allow(unused_macros)]
macro_rules! syscall_always_asm {
    ($nr:ident) => {
        $crate::backend::arch::asm::syscall0($crate::backend::reg::nr(linux_raw_sys::general::$nr))
    };

    ($nr:ident, $a0:expr) => {
        $crate::backend::arch::asm::syscall1(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr) => {
        $crate::backend::arch::asm::syscall2(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr) => {
        $crate::backend::arch::asm::syscall3(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {
        $crate::backend::arch::asm::syscall4(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
        $crate::backend::arch::asm::syscall5(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
        $crate::backend::arch::asm::syscall6(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
            $a5.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
        $crate::backend::arch::asm::syscall7(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
            $a5.into(),
            $a6.into(),
        )
    };
}

/// Like `syscall`, but adds the `readonly` attribute to the inline asm, which
/// indicates that the syscall does not mutate any memory.
macro_rules! syscall_readonly {
    ($nr:ident) => {
        $crate::backend::arch::choose::syscall0_readonly($crate::backend::reg::nr(
            linux_raw_sys::general::$nr,
        ))
    };

    ($nr:ident, $a0:expr) => {
        $crate::backend::arch::choose::syscall1_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr) => {
        $crate::backend::arch::choose::syscall2_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr) => {
        $crate::backend::arch::choose::syscall3_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {
        $crate::backend::arch::choose::syscall4_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {
        $crate::backend::arch::choose::syscall5_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {
        $crate::backend::arch::choose::syscall6_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
            $a5.into(),
        )
    };

    ($nr:ident, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {
        $crate::backend::arch::choose::syscall7_readonly(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
            $a1.into(),
            $a2.into(),
            $a3.into(),
            $a4.into(),
            $a5.into(),
            $a6.into(),
        )
    };
}

/// Like `syscall`, but indicates that the syscall does not return.
#[cfg(feature = "runtime")]
macro_rules! syscall_noreturn {
    ($nr:ident, $a0:expr) => {
        $crate::backend::arch::choose::syscall1_noreturn(
            $crate::backend::reg::nr(linux_raw_sys::general::$nr),
            $a0.into(),
        )
    };
}
