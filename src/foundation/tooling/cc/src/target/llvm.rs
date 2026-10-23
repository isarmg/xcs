use std::borrow::Cow;

use super::{generated, TargetInfo};

impl TargetInfo<'_> {
    /// The LLVM/Clang target triple.
    ///
    /// See <https://clang.llvm.org/docs/CrossCompilation.html#target-triple>.
    ///
    /// Rust and Clang don't really agree on target naming, so we first try to
    /// find the matching trible based on `rustc`'s output, but if no such
    /// triple exists, we attempt to construct the triple from scratch.
    ///
    /// NOTE: You should never need to match on this explicitly, use the
    /// fields on [`TargetInfo`] instead.
    pub(crate) fn llvm_target(
        &self,
        rustc_target: &str,
        version: Option<&str>,
    ) -> Cow<'static, str> {
        if rustc_target == "armv7-apple-ios" {
            // FIXME(madsmtm): Unnecessary once we bump MSRV to Rust 1.74
            return Cow::Borrowed("armv7-apple-ios");
        } else if rustc_target == "aarch64-unknown-linux-pauthtest" {
            // `aarch64-unknown-linux-pauthtest` rustc target sets both
            // environment and abi (to `musl` and `pauthtest` respectively`).
            // However, the abi field exists mainly for `cfg(...)` evaluation in
            // Rust, not for generating the actual LLVM triple. The logic of:
            // `arch-vendor-os-env+abi` does not applay here.
            return Cow::Borrowed("aarch64-unknown-linux-pauthtest");
        } else if self.os == "uefi" {
            // Override the UEFI LLVM targets.
            //
            // The rustc mappings (as of 1.82) for the UEFI targets are:
            // * i686-unknown-uefi -> i686-unknown-windows-gnu
            // * x86_64-unknown-uefi -> x86_64-unknown-windows
            // * aarch64-unknown-uefi -> aarch64-unknown-windows
            //
            // However, in cc-rs all the UEFI targets use
            // -windows-gnu. This has been the case since 2021 [1].
            // * i686-unknown-uefi -> i686-unknown-windows-gnu
            // * x86_64-unknown-uefi -> x86_64-unknown-windows-gnu
            // * aarch64-unknown-uefi -> aarch64-unknown-windows-gnu
            //
            // For now, override the UEFI mapping to keep the behavior
            // of cc-rs unchanged.
            //
            // TODO: as discussed in [2], it may be possible to switch
            // to new UEFI targets added to clang, and regardless it
            // would be good to have consistency between rustc and
            // cc-rs.
            //
            // [1]: https://github.com/rust-lang/cc-rs/pull/623
            // [2]: https://github.com/rust-lang/cc-rs/pull/1264
            return Cow::Owned(format!("{}-unknown-windows-gnu", self.full_arch));
        }

        // If no version is requested, let's take the triple directly from
        // `rustc` (the logic below is not yet good enough for most targets).
        //
        // FIXME(madsmtm): This should ideally be removed.
        if version.is_none() {
            if let Ok(index) = generated::LLVM_TARGETS
                .binary_search_by_key(&rustc_target, |(rustc_target, _)| rustc_target)
            {
                let (_, llvm_target) = &generated::LLVM_TARGETS[index];
                return Cow::Borrowed(llvm_target);
            }
        }

        // Otherwise, attempt to construct the triple from the target info.

        let arch = match self.full_arch {
            riscv32 if riscv32.starts_with("riscv32") => "riscv32",
            riscv64 if riscv64.starts_with("riscv64") => "riscv64",
            "aarch64" if self.vendor == "apple" => "arm64",
            "armv7" if self.vendor == "sony" => "thumbv7a", // FIXME
            arch => arch,
        };
        let vendor = match self.vendor {
            "kmc" | "nintendo" => "unknown",
            "unknown" if self.os == "android" => "linux",
            "uwp" => "pc",
            "espressif" => "",
            _ if self.arch == "msp430" => "",
            vendor => vendor,
        };
        let os = match self.os {
            "macos" => "macosx",
            "visionos" => "xros",
            "uefi" => "windows",
            "solid_asp3" | "horizon" | "teeos" | "nuttx" | "espidf" => "none",
            "qnx" | "nto" => "unknown", // LLVM doesn't know about QNX
            "trusty" => "unknown",      // FIXME
            os => os,
        };
        let version = version.unwrap_or("");
        let env = match self.env {
            "newlib" | "nto70" | "nto71" | "nto71_iosock" | "p1" | "p2" | "relibc" | "sgx"
            | "uclibc" => "",
            "sim" => "simulator",
            env => env,
        };
        let abi = match self.abi {
            "llvm" | "softfloat" | "uwp" | "vec-extabi" => "",
            "ilp32" => "_ilp32",
            "abi64" => "",
            "elfv1" | "elfv2" => "",
            abi => abi,
        };
        Cow::Owned(match (vendor, env, abi) {
            ("", "", "") => format!("{arch}-{os}{version}"),
            ("", env, abi) => format!("{arch}-{os}{version}-{env}{abi}"),
            (vendor, "", "") => format!("{arch}-{vendor}-{os}{version}"),
            (vendor, env, abi) => format!("{arch}-{vendor}-{os}{version}-{env}{abi}"),
        })
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/cc/unit/src/target/llvm/tests_120.rs"));
