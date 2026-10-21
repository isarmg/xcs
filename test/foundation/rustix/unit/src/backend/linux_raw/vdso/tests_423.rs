#[cfg(test)]
mod tests {
    use super::*;

    // Disable on MIPS since QEMU on MIPS doesn't provide a vDSO.
    #[cfg(linux_raw)]
    #[test]
    #[cfg_attr(any(target_arch = "mips", target_arch = "mips64"), ignore)]
    #[allow(unused_variables)]
    fn test_vdso() {
        let vdso = Vdso::new().unwrap();
        assert!(!vdso.symtab.is_null());
        assert!(!vdso.symstrings.is_null());

        {
            #[cfg(target_arch = "x86_64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_gettime"));
            #[cfg(target_arch = "arm")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_gettime64"));
            #[cfg(target_arch = "aarch64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.39"), cstr!("__kernel_clock_gettime"));
            #[cfg(target_arch = "x86")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_gettime64"));
            #[cfg(target_arch = "riscv64")]
            let ptr = vdso.sym(cstr!("LINUX_4.15"), cstr!("__vdso_clock_gettime"));
            #[cfg(target_arch = "powerpc")]
            let _ptr = vdso.sym(cstr!("LINUX_5.11"), cstr!("__kernel_clock_gettime64"));
            #[cfg(target_arch = "powerpc64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.15"), cstr!("__kernel_clock_gettime"));
            #[cfg(target_arch = "s390x")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.29"), cstr!("__kernel_clock_gettime"));
            #[cfg(any(target_arch = "mips", target_arch = "mips32r6"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_gettime64"));
            #[cfg(any(target_arch = "mips64", target_arch = "mips64r6"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_gettime"));

            // On PowerPC, "__kernel_clock_gettime64" isn't available in
            // Linux < 5.11.
            // On x86, "__vdso_clock_gettime64" isn't available in
            // Linux < 5.3.
            #[cfg(not(any(target_arch = "powerpc", target_arch = "x86")))]
            assert!(!ptr.is_null());
        }

        {
            #[cfg(target_arch = "x86_64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_getres"));
            #[cfg(target_arch = "arm")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_getres"));
            #[cfg(target_arch = "aarch64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.39"), cstr!("__kernel_clock_getres"));
            #[cfg(target_arch = "x86")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_getres"));
            #[cfg(target_arch = "riscv64")]
            let ptr = vdso.sym(cstr!("LINUX_4.15"), cstr!("__vdso_clock_getres"));
            #[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6.15"), cstr!("__kernel_clock_getres"));
            #[cfg(target_arch = "s390x")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.29"), cstr!("__kernel_clock_getres"));
            #[cfg(any(target_arch = "mips", target_arch = "mips32r6"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_getres"));
            #[cfg(any(target_arch = "mips64", target_arch = "mips64r6"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_clock_getres"));

            // Some versions of Linux appear to lack "__vdso_clock_getres" on x86.
            #[cfg(not(target_arch = "x86"))]
            assert!(!ptr.is_null());
        }

        {
            #[cfg(target_arch = "x86_64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_gettimeofday"));
            #[cfg(target_arch = "arm")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_gettimeofday"));
            #[cfg(target_arch = "aarch64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.39"), cstr!("__kernel_gettimeofday"));
            #[cfg(target_arch = "x86")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_gettimeofday"));
            #[cfg(target_arch = "riscv64")]
            let ptr = vdso.sym(cstr!("LINUX_4.15"), cstr!("__vdso_gettimeofday"));
            #[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6.15"), cstr!("__kernel_gettimeofday"));
            #[cfg(target_arch = "s390x")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.29"), cstr!("__kernel_gettimeofday"));
            #[cfg(any(target_arch = "mips", target_arch = "mips32r6"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_gettimeofday"));
            #[cfg(any(target_arch = "mips64", target_arch = "mips64r6"))]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_gettimeofday"));

            // Some versions of Linux appear to lack "__vdso_gettimeofday" on x86.
            #[cfg(not(target_arch = "x86"))]
            assert!(!ptr.is_null());
        }

        #[cfg(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "riscv64",
            target_arch = "powerpc",
            target_arch = "powerpc64",
            target_arch = "s390x",
        ))]
        {
            #[cfg(target_arch = "x86_64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_getcpu"));
            #[cfg(target_arch = "x86")]
            let ptr = vdso.sym(cstr!("LINUX_2.6"), cstr!("__vdso_getcpu"));
            #[cfg(target_arch = "riscv64")]
            let ptr = vdso.sym(cstr!("LINUX_4.15"), cstr!("__vdso_getcpu"));
            #[cfg(target_arch = "powerpc")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.15"), cstr!("__kernel_getcpu"));
            #[cfg(target_arch = "powerpc64")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.15"), cstr!("__kernel_getcpu"));
            #[cfg(target_arch = "s390x")]
            let ptr = vdso.sym(cstr!("LINUX_2.6.29"), cstr!("__kernel_getcpu"));

            // On PowerPC, "__kernel_getcpu" isn't available in 32-bit kernels.
            // Some versions of Linux appear to lack "__vdso_getcpu" on x86.
            #[cfg(not(any(target_arch = "powerpc", target_arch = "x86")))]
            assert!(!ptr.is_null());
        }
    }
}
