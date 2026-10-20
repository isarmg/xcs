use std::env::var;
use std::io::Write as _;
use std::path::PathBuf;

/// The directory for inline asm.
const ASM_PATH: &str = "src/backend/linux_raw/arch";

fn main() {
    // Don't rerun this on changes other than build.rs, as we only depend on
    // the rustc version.
    println!("cargo:rerun-if-changed=build.rs");

    // Native platform support is Linux amd64 LP64. Keep the libc backend for
    // use-libc and Miri, including future GNU/musl/uclibc ABI configurations.
    // Browser wasm uses separate components and never compiles this crate.
    let arch = var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = var("CARGO_CFG_TARGET_OS").unwrap();
    let pointer_width = var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    assert!(
        supports_target(&os, &arch, &pointer_width),
        "rustix supports only Linux amd64 with 64-bit pointers"
    );
    assert!(
        std::path::Path::new(ASM_PATH).join("x86_64.rs").is_file(),
        "the maintained Linux amd64 syscall implementation is missing"
    );

    // Check for `--features=use-libc`. This allows crate users to enable the
    // libc backend.
    let feature_use_libc = var("CARGO_FEATURE_USE_LIBC").is_ok();

    // Check for `RUSTFLAGS=--cfg=rustix_use_libc`. This allows end users to
    // enable the libc backend even if rustix is depended on transitively.
    let cfg_use_libc = var("CARGO_CFG_RUSTIX_USE_LIBC").is_ok();

    // Check for `RUSTFLAGS=--cfg=rustix_no_linux_raw`. This allows Linux users to
    // enable the libc backend without the linux raw dependency.
    let cfg_no_linux_raw = var("CARGO_CFG_RUSTIX_NO_LINUX_RAW").is_ok();

    // Check for `--features=rustc-dep-of-std`.
    let rustc_dep_of_std = var("CARGO_FEATURE_RUSTC_DEP_OF_STD").is_ok();

    // Check for eg. `RUSTFLAGS=--cfg=rustix_use_experimental_features`. This
    // is a rustc flag rather than a cargo feature flag because it's
    // experimental and not something we want accidentally enabled via
    // `--all-features`.
    let rustix_use_experimental_features =
        var("CARGO_CFG_RUSTIX_USE_EXPERIMENTAL_FEATURES").is_ok();

    // Check for eg. `RUSTFLAGS=--cfg=rustix_use_experimental_asm`. This is a
    // rustc flag rather than a cargo feature flag because it's experimental
    // and not something we want accidentally enabled via `--all-features`.
    let rustix_use_experimental_asm = var("CARGO_CFG_RUSTIX_USE_EXPERIMENTAL_ASM").is_ok();

    // Miri doesn't support inline asm, and has builtin support for recognizing
    // libc FFI calls, so if we're running under miri, use the libc backend.
    let miri = var("CARGO_CFG_MIRI").is_ok();

    // If experimental features are enabled, auto-detect and use available
    // features.
    if rustc_dep_of_std {
        use_feature("core_intrinsics");
    } else if rustix_use_experimental_features {
        use_feature_or_nothing("core_intrinsics");
    }

    // Features needed only in no-std configurations.
    #[cfg(not(feature = "std"))]
    {
        use_feature_or_nothing("core_c_str");
        use_feature_or_nothing("core_ffi_c");
        use_feature_or_nothing("alloc_c_string");
        use_feature_or_nothing("alloc_ffi");
        use_feature_or_nothing("error_in_core");
    }

    // `LowerExp`/`UpperExp` for `NonZeroI32` etc.
    if has_lower_upper_exp_for_non_zero() {
        use_feature("lower_upper_exp_for_non_zero");
    }

    if can_compile("#[diagnostic::on_unimplemented()] trait Foo {}") {
        use_feature("rustc_diagnostics")
    }

    // The native assembly implementation is fixed, but users and Miri may
    // select the retained libc backend exactly as before.
    let libc = feature_use_libc || cfg_use_libc || miri;
    if libc {
        if !cfg_no_linux_raw {
            use_feature("linux_raw_dep");
        }

        // Use the libc backend.
        use_feature("libc");
    } else {
        // Use the linux_raw backend.
        use_feature("linux_raw_dep");
        use_feature("linux_raw");
        if rustix_use_experimental_asm {
            use_feature("asm_experimental_arch");
        }
    }

    // Both retained backends target the same Linux kernel/API family.
    use_feature("linux_like");
    use_feature("linux_kernel");

    println!("cargo:rerun-if-env-changed=CARGO_CFG_RUSTIX_USE_EXPERIMENTAL_ASM");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_RUSTIX_USE_LIBC");

    // Rerun this script if any of our features or configuration flags change,
    // or if the toolchain we used for feature detection changes.
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_USE_LIBC");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_RUSTC_DEP_OF_STD");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_MIRI");

    // Remove the can_compile() probe artifact so OUT_DIR ends up empty.
    // The emitted metadata is not byte-reproducible, and a nondeterministic
    // OUT_DIR destabilizes the cache keys of some build systems like Bazel.
    let _ = std::fs::remove_file(probe_file());
}

fn has_lower_upper_exp_for_non_zero() -> bool {
    // LowerExp/UpperExp for NonZero* were added in Rust 1.84.
    // <https://doc.rust-lang.org/stable/std/fmt/trait.LowerExp.html#impl-LowerExp-for-NonZero%3CT%3E>
    can_compile("fn a(x: &core::num::NonZeroI32, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { core::fmt::LowerExp::fmt(x, f) }")
}

fn supports_target(os: &str, arch: &str, pointer_width: &str) -> bool {
    os == "linux" && arch == "x86_64" && pointer_width == "64"
}

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

fn use_feature_or_nothing(feature: &str) {
    if has_feature(feature) {
        use_feature(feature);
    }
}

fn use_feature(feature: &str) {
    println!("cargo:rustc-cfg={}", feature);
}

/// Test whether the rustc at `var("RUSTC")` supports the given feature.
fn has_feature(feature: &str) -> bool {
    can_compile(format!(
        "#![allow(stable_features)]\n#![feature({})]",
        feature
    ))
}

fn probe_file() -> PathBuf {
    PathBuf::from(var("OUT_DIR").unwrap()).join("rustix_test_can_compile")
}

/// Test whether the rustc at `var("RUSTC")` can compile the given code.
fn can_compile<T: AsRef<str>>(test: T) -> bool {
    use std::process::Stdio;

    let rustc = var("RUSTC").unwrap();
    let target = var("TARGET").unwrap();

    // Use `RUSTC_WRAPPER` if it's set, unless it's set to an empty string, as
    // documented [here].
    // [here]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-reads
    let wrapper = var("RUSTC_WRAPPER")
        .ok()
        .and_then(|w| if w.is_empty() { None } else { Some(w) });

    let mut cmd = if let Some(wrapper) = wrapper {
        let mut cmd = std::process::Command::new(wrapper);
        // The wrapper's first argument is supposed to be the path to rustc.
        cmd.arg(rustc);
        cmd
    } else {
        std::process::Command::new(rustc)
    };

    cmd.arg("--crate-type=rlib") // Don't require `main`.
        .arg("--emit=metadata") // Do as little as possible but still parse.
        .arg("--target")
        .arg(target)
        .arg("-o")
        .arg(probe_file())
        .stdout(Stdio::null()); // We don't care about the output (only whether it builds or not)

    // If Cargo wants to set RUSTFLAGS, use that.
    if let Ok(rustflags) = var("CARGO_ENCODED_RUSTFLAGS") {
        if !rustflags.is_empty() {
            for arg in rustflags.split('\x1f') {
                cmd.arg(arg);
            }
        }
    }

    let mut child = cmd
        .arg("-") // Read from stdin.
        .stdin(Stdio::piped()) // Stdin is a pipe.
        .stderr(Stdio::null()) // Errors from feature detection aren't interesting and can be confusing.
        .spawn()
        .unwrap();

    writeln!(child.stdin.take().unwrap(), "{}", test.as_ref()).unwrap();

    child.wait().unwrap().success()
}
