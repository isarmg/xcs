#[cfg(test)]
mod platform_tests {
    #[test]
    fn native_scope_is_linux_amd64_lp64_not_a_libc_vendor() {
        assert!(super::supports_target("linux", "x86_64", "64"));
        for (os, arch, width) in [
            ("linux", "aarch64", "64"),
            ("linux", "x86", "32"),
            ("linux", "x86_64", "32"),
            ("windows", "x86_64", "64"),
            ("macos", "x86_64", "64"),
            ("unknown", "wasm32", "32"),
        ] {
            assert!(!super::supports_target(os, arch, width));
        }
    }
}
