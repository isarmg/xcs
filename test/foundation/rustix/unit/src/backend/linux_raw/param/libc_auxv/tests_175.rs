#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi() {
        static_assertions::const_assert_eq!(self::_SC_PAGESIZE, ::libc::_SC_PAGESIZE);
        static_assertions::const_assert_eq!(self::_SC_CLK_TCK, ::libc::_SC_CLK_TCK);
        static_assertions::const_assert_eq!(self::AT_HWCAP, ::libc::AT_HWCAP);
        static_assertions::const_assert_eq!(self::AT_HWCAP2, ::libc::AT_HWCAP2);
        static_assertions::const_assert_eq!(self::AT_EXECFN, ::libc::AT_EXECFN);
        static_assertions::const_assert_eq!(self::AT_SECURE, ::libc::AT_SECURE);
        static_assertions::const_assert_eq!(self::AT_SYSINFO_EHDR, ::libc::AT_SYSINFO_EHDR);
        static_assertions::const_assert_eq!(self::AT_MINSIGSTKSZ, ::libc::AT_MINSIGSTKSZ);
        #[cfg(feature = "runtime")]
        static_assertions::const_assert_eq!(self::AT_PHDR, ::libc::AT_PHDR);
        #[cfg(feature = "runtime")]
        static_assertions::const_assert_eq!(self::AT_PHNUM, ::libc::AT_PHNUM);
        #[cfg(feature = "runtime")]
        static_assertions::const_assert_eq!(self::AT_ENTRY, ::libc::AT_ENTRY);
        #[cfg(feature = "runtime")]
        static_assertions::const_assert_eq!(self::AT_RANDOM, ::libc::AT_RANDOM);
    }
}
