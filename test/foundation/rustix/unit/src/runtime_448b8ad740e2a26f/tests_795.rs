#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assumptions() {
        assert!(libc::SIGSYS < KERNEL_SIGRTMIN);
        assert!(KERNEL_SIGRTMIN <= libc::SIGRTMIN());

        // POSIX guarantees at least 8 RT signals.
        assert!(libc::SIGRTMIN() + 8 <= KERNEL_SIGRTMAX);

        // POSIX guarantees at least 8 RT signals, and it's not uncommon for
        // libc implementations to reserve up to 3 for their own purposes.
        assert!(KERNEL_SIGRTMIN + 8 + 3 <= KERNEL_SIGRTMAX);

        assert!(KERNEL_SIGRTMAX <= libc::SIGRTMAX());
        assert!(libc::SIGRTMAX() as u32 <= linux_raw_sys::general::_NSIG);

        assert!(KERNEL_SIGRTMAX as usize - 1 < core::mem::size_of::<KernelSigSet>() * 8);
    }

    #[test]
    fn test_layouts_matching_libc() {
        use linux_raw_sys::general::siginfo__bindgen_ty_1__bindgen_ty_1;

        // c-scape assumes rustix's `Siginfo` matches libc's. We don't use
        // check_types macros because we want to test compatibility with actual
        // libc, not the `crate::backend::c` which might be our own
        // implementation.
        static_assertions::assert_eq_size!(Siginfo, libc::siginfo_t);
        static_assertions::assert_eq_align!(Siginfo, libc::siginfo_t);
        assert_eq!(
            memoffset::span_of!(Siginfo, ..),
            memoffset::span_of!(Siginfo, __bindgen_anon_1)
        );
        assert_eq!(
            memoffset::span_of!(siginfo__bindgen_ty_1__bindgen_ty_1, si_signo),
            memoffset::span_of!(libc::siginfo_t, si_signo)
        );
        assert_eq!(
            memoffset::span_of!(siginfo__bindgen_ty_1__bindgen_ty_1, si_errno),
            memoffset::span_of!(libc::siginfo_t, si_errno)
        );
        assert_eq!(
            memoffset::span_of!(siginfo__bindgen_ty_1__bindgen_ty_1, si_code),
            memoffset::span_of!(libc::siginfo_t, si_code)
        );

        // c-scape assumes rustix's `Stack` matches libc's. Similar to above.
        static_assertions::assert_eq_size!(Stack, libc::stack_t);
        static_assertions::assert_eq_align!(Stack, libc::stack_t);
        assert_eq!(
            memoffset::span_of!(Stack, ss_sp),
            memoffset::span_of!(libc::stack_t, ss_sp)
        );
        assert_eq!(
            memoffset::span_of!(Stack, ss_flags),
            memoffset::span_of!(libc::stack_t, ss_flags)
        );
        assert_eq!(
            memoffset::span_of!(Stack, ss_size),
            memoffset::span_of!(libc::stack_t, ss_size)
        );
    }

    #[test]
    fn test_layouts_matching_kernel() {
        use linux_raw_sys::general as c;

        // Rustix's versions of these must match the kernel's versions.
        // Some architectures have `sa_restorer`.
        #[cfg(not(any(
            target_arch = "csky",
            target_arch = "loongarch64",
            target_arch = "mips",
            target_arch = "mips32r6",
            target_arch = "mips64",
            target_arch = "mips64r6",
            target_arch = "riscv32",
            target_arch = "riscv64"
        )))]
        check_renamed_struct!(
            KernelSigaction,
            kernel_sigaction,
            sa_handler_kernel,
            sa_flags,
            sa_restorer,
            sa_mask
        );
        // Some architectures omit `sa_restorer`.
        #[cfg(any(
            target_arch = "csky",
            target_arch = "loongarch64",
            target_arch = "mips",
            target_arch = "mips32r6",
            target_arch = "mips64",
            target_arch = "mips64r6",
            target_arch = "riscv32",
            target_arch = "riscv64"
        ))]
        check_renamed_struct!(
            KernelSigaction,
            kernel_sigaction,
            sa_handler_kernel,
            sa_flags,
            sa_mask
        );
        static_assertions::assert_eq_size!(KernelSigactionFlags, crate::ffi::c_ulong);
        static_assertions::assert_eq_align!(KernelSigactionFlags, crate::ffi::c_ulong);
        check_renamed_type!(KernelSigrestore, __sigrestore_t);
        check_renamed_type!(KernelSighandler, __kernel_sighandler_t);

        assert_eq!(
            libc::SA_NOCLDSTOP,
            KernelSigactionFlags::NOCLDSTOP.bits() as _
        );
        assert_eq!(
            libc::SA_NOCLDWAIT,
            KernelSigactionFlags::NOCLDWAIT.bits() as _
        );
        assert_eq!(libc::SA_NODEFER, KernelSigactionFlags::NODEFER.bits() as _);
        assert_eq!(libc::SA_ONSTACK, KernelSigactionFlags::ONSTACK.bits() as _);
        assert_eq!(
            libc::SA_RESETHAND,
            KernelSigactionFlags::RESETHAND.bits() as _
        );
        assert_eq!(libc::SA_RESTART, KernelSigactionFlags::RESTART.bits() as _);
        assert_eq!(libc::SA_SIGINFO, KernelSigactionFlags::SIGINFO.bits() as _);
    }
}
