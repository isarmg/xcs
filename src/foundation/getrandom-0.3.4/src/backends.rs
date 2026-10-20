//! System-specific implementations.
//!
//! This module should provide `fill_inner` with the signature
//! `fn fill_inner(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error>`.
//! The function MUST fully initialize `dest` when `Ok(())` is returned;
//! the function may need to use `sanitizer::unpoison` as well.
//! The function MUST NOT ever write uninitialized bytes into `dest`,
//! regardless of what value it returns.

cfg_if! {
    if #[cfg(getrandom_backend = "custom")] {
        mod custom;
        pub use custom::*;
    } else if #[cfg(getrandom_backend = "linux_getrandom")] {
        mod getrandom;
        mod sanitizer;
        pub use getrandom::*;
    } else if #[cfg(getrandom_backend = "linux_raw")] {
        mod linux_raw;
        mod sanitizer;
        pub use linux_raw::*;
    } else if #[cfg(getrandom_backend = "rdrand")] {
        mod rdrand;
        pub use rdrand::*;
    } else if #[cfg(any(
        getrandom_backend = "rndr",
        getrandom_backend = "efi_rng",
        getrandom_backend = "windows_legacy",
    ))] {
        compile_error!("this getrandom backend was retired: only Linux amd64 and browser WebAssembly backends are retained");
        mod unsupported;
        pub use unsupported::*;
    } else if #[cfg(getrandom_backend = "wasm_js")] {
        cfg_if! {
            if #[cfg(feature = "wasm_js")] {
                mod wasm_js;
                pub use wasm_js::*;
            } else {
                compile_error!(concat!(
                    "The \"wasm_js\" backend requires the `wasm_js` feature \
                    for `getrandom`. For more information see: \
                    https://docs.rs/getrandom/", env!("CARGO_PKG_VERSION"), "/#webassembly-support"
                ));
            }
        }
    } else if #[cfg(getrandom_backend = "unsupported")] {
        mod unsupported;
        pub use unsupported::*;
    } else if #[cfg(all(target_os = "linux", target_env = ""))] {
        mod linux_raw;
        mod sanitizer;
        pub use linux_raw::*;
    } else if #[cfg(any(
        target_os = "haiku",
        target_os = "redox",
        target_os = "nto",
        target_os = "aix",
    ))] {
        mod use_file;
        pub use use_file::*;
    } else if #[cfg(any(
        // Rust supports Android API level 19 (KitKat) [0] and the next upgrade targets
        // level 21 (Lollipop) [1], while `getrandom(2)` was added only in
        // level 23 (Marshmallow). Note that it applies only to the "old" `target_arch`es,
        // RISC-V Android targets sufficiently new API level, same will apply for potential
        // new Android `target_arch`es.
        // [0]: https://blog.rust-lang.org/2023/01/09/android-ndk-update-r25.html
        // [1]: https://github.com/rust-lang/rust/pull/120593
        all(
            target_os = "android",
            any(
                target_arch = "aarch64",
                target_arch = "arm",
                target_arch = "x86",
                target_arch = "x86_64",
            ),
        ),
        // Only on these `target_arch`es Rust supports Linux kernel versions (3.2+)
        // that precede the version (3.17) in which `getrandom(2)` was added:
        // https://doc.rust-lang.org/stable/rustc/platform-support.html
        all(
            target_os = "linux",
            any(
                target_arch = "aarch64",
                target_arch = "arm",
                target_arch = "powerpc",
                target_arch = "powerpc64",
                target_arch = "s390x",
                target_arch = "x86",
                target_arch = "x86_64",
                // Minimum supported Linux kernel version for MUSL targets
                // is not specified explicitly (as of Rust 1.77) and they
                // are used in practice to target pre-3.17 kernels.
                all(
                    target_env = "musl",
                    not(
                        any(
                            target_arch = "riscv64",
                            target_arch = "riscv32",
                        ),
                    ),
                ),
            ),
        )
    ))] {
        mod use_file;
        mod linux_android_with_fallback;
        mod sanitizer;
        pub use linux_android_with_fallback::*;
    } else if #[cfg(any(
        target_os = "android",
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "hurd",
        target_os = "illumos",
        target_os = "cygwin",
        // Check for target_arch = "arm" to only include the 3DS. Does not
        // include the Nintendo Switch (which is target_arch = "aarch64").
        all(target_os = "horizon", target_arch = "arm"),
    ))] {
        mod getrandom;
        #[cfg(any(target_os = "android", target_os = "linux"))]
        mod sanitizer;
        pub use getrandom::*;
    } else if #[cfg(all(target_arch = "x86_64", target_env = "sgx"))] {
        mod rdrand;
        pub use rdrand::*;
    } else if #[cfg(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")))] {
        cfg_if! {
            if #[cfg(feature = "wasm_js")] {
                mod wasm_js;
                pub use wasm_js::*;
            } else {
                compile_error!(concat!(
                    "The wasm32-unknown-unknown targets are not supported by default; \
                    you may need to enable the \"wasm_js\" configuration flag. Note \
                    that enabling the `wasm_js` feature flag alone is insufficient. \
                    For more information see: \
                    https://docs.rs/getrandom/", env!("CARGO_PKG_VERSION"), "/#webassembly-support"
                ));
            }
        }
    } else {
        compile_error!(concat!(
            "target is not supported. You may need to define a custom backend see: \
            https://docs.rs/getrandom/", env!("CARGO_PKG_VERSION"), "/#custom-backend"
        ));
    }
}
