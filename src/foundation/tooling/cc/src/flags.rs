use crate::target::TargetInfo;
use crate::{Build, CompilerFlag, Error, ErrorKind, Tool, ToolFamily};
use std::borrow::Cow;
use std::ffi::OsString;

#[derive(Debug, PartialEq, Default)]
pub(crate) struct RustcCodegenFlags<'a> {
    branch_protection: Option<&'a str>,
    code_model: Option<&'a str>,
    no_vectorize_loops: bool,
    no_vectorize_slp: bool,
    control_flow_guard: Option<&'a str>,
    lto: Option<&'a str>,
    relocation_model: Option<&'a str>,
    embed_bitcode: Option<bool>,
    force_frame_pointers: Option<bool>,
    no_redzone: Option<bool>,
    soft_float: Option<bool>,
    dwarf_version: Option<u32>,
    stack_protector: Option<&'a str>,
    linker_plugin_lto: Option<bool>,
    target_features: Vec<&'a str>,
}

impl<'this> RustcCodegenFlags<'this> {
    // Parse flags obtained from CARGO_ENCODED_RUSTFLAGS
    pub(crate) fn parse(rustflags_env: &'this str) -> Result<Self, Error> {
        fn is_flag_prefix(flag: &str) -> bool {
            [
                "-Z",
                "-C",
                "--codegen",
                "-L",
                "-l",
                "-o",
                "-W",
                "--warn",
                "-A",
                "--allow",
                "-D",
                "--deny",
                "-F",
                "--forbid",
            ]
            .contains(&flag)
        }

        fn handle_flag_prefix<'a>(prev: &'a str, curr: &'a str) -> (&'a str, &'a str) {
            match prev {
                "--codegen" | "-C" => ("-C", curr),
                // Handle flags passed like --codegen=code-model=small
                _ if curr.starts_with("--codegen=") => ("-C", &curr[10..]),
                "-Z" => ("-Z", curr),
                "-L" | "-l" | "-o" => (prev, curr),
                // Handle lint flags
                "-W" | "--warn" => ("-W", curr),
                "-A" | "--allow" => ("-A", curr),
                "-D" | "--deny" => ("-D", curr),
                "-F" | "--forbid" => ("-F", curr),
                _ => ("", curr),
            }
        }

        let mut codegen_flags = Self::default();

        let mut prev_prefix = None;
        for curr in rustflags_env.split("\u{1f}") {
            let prev = prev_prefix.take().unwrap_or("");
            if prev.is_empty() && is_flag_prefix(curr) {
                prev_prefix = Some(curr);
                continue;
            }

            let (prefix, rustc_flag) = handle_flag_prefix(prev, curr);
            codegen_flags.set_rustc_flag(prefix, rustc_flag)?;
        }

        Ok(codegen_flags)
    }

    fn set_rustc_flag(&mut self, prefix: &str, flag: &'this str) -> Result<(), Error> {
        // Convert a textual representation of a bool-like rustc flag argument into an actual bool
        fn arg_to_bool(arg: impl AsRef<str>) -> Option<bool> {
            match arg.as_ref() {
                "y" | "yes" | "on" | "true" => Some(true),
                "n" | "no" | "off" | "false" => Some(false),
                _ => None,
            }
        }

        fn arg_to_u32(arg: impl AsRef<str>) -> Option<u32> {
            arg.as_ref().parse().ok()
        }

        let (flag, value) = if let Some((flag, value)) = flag.split_once('=') {
            (flag, Some(value))
        } else {
            (flag, None)
        };
        let flag = if prefix.is_empty() {
            Cow::Borrowed(flag)
        } else {
            Cow::Owned(format!("{prefix}{flag}"))
        };
        let flag = flag.as_ref();

        fn flag_not_empty_generic<T>(
            flag: &str,
            flag_value: Option<T>,
        ) -> Result<Option<T>, Error> {
            if let Some(flag_value) = flag_value {
                Ok(Some(flag_value))
            } else {
                Err(Error::new(
                    ErrorKind::InvalidFlag,
                    format!("{flag} must have a value"),
                ))
            }
        }
        let flag_not_empty = |flag_value| flag_not_empty_generic(flag, flag_value);

        match flag {
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#code-model
            "-Ccode-model" => {
                self.code_model = flag_not_empty(value)?;
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#no-vectorize-loops
            "-Cno-vectorize-loops" => self.no_vectorize_loops = true,
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#no-vectorize-slp
            "-Cno-vectorize-slp" => self.no_vectorize_slp = true,
            // `-Cprofile-generate` and `-Cprofile-use` are deliberately *not* parsed, so that
            // they are never forwarded to the C compiler. See the note in `cc_flags` below.
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#control-flow-guard
            "-Ccontrol-flow-guard" => self.control_flow_guard = value.or(Some("true")),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#lto
            //
            // This variable is currently unused, we just keep it in case we need it in future
            "-Clto" => self.lto = value.or(Some("true")),
            // https://doc.rust-lang.org/rustc/linker-plugin-lto.html
            "-Clinker-plugin-lto" => self.linker_plugin_lto = Some(true),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#relocation-model
            "-Crelocation-model" => {
                self.relocation_model = flag_not_empty(value)?;
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#embed-bitcode
            "-Cembed-bitcode" => self.embed_bitcode = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#force-frame-pointers
            "-Cforce-frame-pointers" => {
                self.force_frame_pointers = value.map_or(Some(true), arg_to_bool)
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#no-redzone
            "-Cno-redzone" => self.no_redzone = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#soft-float
            // Note: This flag is now deprecated in rustc.
            "-Csoft-float" => self.soft_float = value.map_or(Some(true), arg_to_bool),
            // https://doc.rust-lang.org/beta/unstable-book/compiler-flags/branch-protection.html
            // FIXME: Drop the -Z variant and update the doc link once the option is stabilised
            "-Zbranch-protection" | "-Cbranch-protection" => {
                self.branch_protection = flag_not_empty(value)?;
            }
            // https://doc.rust-lang.org/beta/rustc/codegen-options/index.html#dwarf-version
            "-Cdwarf-version" => {
                self.dwarf_version = flag_not_empty_generic(flag, value.and_then(arg_to_u32))?;
            }
            // https://github.com/rust-lang/rust/issues/114903
            // FIXME: Drop the -Z variant and update the doc link once the option is stabilized
            "-Zstack-protector" | "-Cstack-protector" => {
                self.stack_protector = flag_not_empty(value)?;
            }
            // https://doc.rust-lang.org/rustc/codegen-options/index.html#target-feature
            "-Ctarget-feature" => {
                if let Some(value) = flag_not_empty(value)? {
                    self.target_features
                        .extend(value.split(',').filter(|feature| !feature.is_empty()));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Map a single `-Ctarget-feature` entry such as `+avx2` to the matching GCC/Clang flag.
    ///
    /// Only features whose GCC/Clang flag is known to mean the same thing are mapped. Anything
    /// else (unknown features, other architectures) is skipped instead of being passed through.
    fn target_feature_cc_flag(arch: &str, feature: &str) -> Option<String> {
        let (enable, feature) = match feature.strip_prefix('+') {
            Some(feature) => (true, feature),
            // rustc ignores features without a `+` or `-` prefix
            None => (false, feature.strip_prefix('-')?),
        };

        let cc_feature = match arch {
            // https://doc.rust-lang.org/reference/attributes/codegen.html#x86-or-x86_64
            // https://gcc.gnu.org/onlinedocs/gcc/x86-Options.html
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#x86
            // `sse` and `sse2` are left out: they are part of the x86 ABI, and `-mno-sse2` makes
            // GCC reject C functions that return floating point values.
            "x86" | "x86_64" => match feature {
                "adx" | "aes" | "avx" | "avx2" | "avx512bf16" | "avx512bitalg" | "avx512bw"
                | "avx512cd" | "avx512dq" | "avx512f" | "avx512fp16" | "avx512ifma"
                | "avx512vbmi" | "avx512vbmi2" | "avx512vl" | "avx512vnni" | "avx512vpopcntdq"
                | "avxvnni" | "bmi2" | "f16c" | "fma" | "fxsr" | "gfni" | "lzcnt" | "movbe"
                | "popcnt" | "rdseed" | "sha" | "sse3" | "sse4.1" | "sse4.2" | "sse4a"
                | "ssse3" | "vaes" | "vpclmulqdq" | "xsave" | "xsavec" | "xsaveopt" | "xsaves" => {
                    feature
                }
                // Rust and GCC/Clang use different names for these
                "bmi1" => "bmi",
                "cmpxchg16b" => "cx16",
                "pclmulqdq" => "pclmul",
                "rdrand" => "rdrnd",
                _ => return None,
            },
            _ => return None,
        };

        Some(if enable {
            format!("-m{cc_feature}")
        } else {
            format!("-mno-{cc_feature}")
        })
    }

    // Rust and clang/cc don't agree on what equivalent flags should look like.
    pub(crate) fn cc_flags(
        &self,
        build: &Build,
        tool: &mut Tool,
        target: &TargetInfo<'_>,
        flag_support_key: &mut Option<CompilerFlag>,
    ) {
        let family = tool.family;
        // Push `flag` to `flags` if it is supported by the currently used CC
        let mut push_if_supported = |flag: OsString| {
            if build
                .is_flag_supported_inner(&flag, tool, target, flag_support_key)
                .unwrap_or(false)
            {
                tool.args.push(flag);
            } else {
                build.cargo_output.print_warning(&format!(
                    "Inherited flag {flag:?} is not supported by the currently used CC"
                ));
            }
        };

        let clang_or_gnu =
            matches!(family, ToolFamily::Clang { .. }) || matches!(family, ToolFamily::Gnu);

        // Flags shared between clang and gnu
        if clang_or_gnu {
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mbranch-protection
            // https://gcc.gnu.org/onlinedocs/gcc/AArch64-Options.html#index-mbranch-protection (Aarch64)
            // https://gcc.gnu.org/onlinedocs/gcc/ARM-Options.html#index-mbranch-protection-1 (ARM)
            // https://developer.arm.com/documentation/101754/0619/armclang-Reference/armclang-Command-line-Options/-mbranch-protection
            if let Some(value) = self.branch_protection {
                push_if_supported(
                    format!("-mbranch-protection={}", value.replace(",", "+")).into(),
                );
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mcmodel
            // https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html (several archs, search for `-mcmodel=`).
            // FIXME(madsmtm): Parse the model, to make sure we pass the correct value (depending on arch).
            if let Some(value) = self.code_model {
                push_if_supported(format!("-mcmodel={value}").into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fno-vectorize
            // https://gcc.gnu.org/onlinedocs/gnat_ugn/Vectorization-of-loops.html
            if self.no_vectorize_loops {
                push_if_supported("-fno-vectorize".into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fno-slp-vectorize
            // https://gcc.gnu.org/onlinedocs/gnat_ugn/Vectorization-of-loops.html
            if self.no_vectorize_slp {
                push_if_supported("-fno-slp-vectorize".into());
            }
            if let Some(value) = self.relocation_model {
                let cc_flag = match value {
                    // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fPIC
                    // https://gcc.gnu.org/onlinedocs/gcc/Code-Gen-Options.html#index-fPIC
                    "pic" => Some("-fPIC"),
                    // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fPIE
                    // https://gcc.gnu.org/onlinedocs/gcc/Code-Gen-Options.html#index-fPIE
                    "pie" => Some("-fPIE"),
                    // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mdynamic-no-pic
                    // https://gcc.gnu.org/onlinedocs/gcc/RS_002f6000-and-PowerPC-Options.html#index-mdynamic-no-pic
                    "dynamic-no-pic" => Some("-mdynamic-no-pic"),
                    _ => None,
                };
                if let Some(cc_flag) = cc_flag {
                    push_if_supported(cc_flag.into());
                }
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fno-omit-frame-pointer
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-momit-leaf-frame-pointer
            // https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html#index-fomit-frame-pointer
            if let Some(value) = self.force_frame_pointers {
                if value {
                    push_if_supported("-fno-omit-frame-pointer".into());
                    push_if_supported("-mno-omit-leaf-frame-pointer".into());
                }
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mno-red-zone
            // https://gcc.gnu.org/onlinedocs/gcc/x86-Options.html#index-mno-red-zone
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mred-zone
            // https://gcc.gnu.org/onlinedocs/gcc/x86-Options.html#index-mred-zone
            if let Some(value) = self.no_redzone {
                let cc_flag = if value { "-mno-red-zone" } else { "-mred-zone" };
                push_if_supported(cc_flag.into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-msoft-float
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mhard-float
            // https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html (several archs, search for `-msoft-float`).
            // https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html (several archs, search for `-mhard-float`).
            if let Some(value) = self.soft_float {
                let cc_flag = if value {
                    "-msoft-float"
                } else {
                    // Do not use -mno-soft-float, that's basically just an alias for -mno-implicit-float.
                    "-mhard-float"
                };
                push_if_supported(cc_flag.into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-gdwarf-2
            // https://gcc.gnu.org/onlinedocs/gcc/Debugging-Options.html#index-gdwarf
            if let Some(value) = self.dwarf_version {
                push_if_supported(format!("-gdwarf-{value}").into());
            }
            // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fstack-protector
            // https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html#index-fstack-protector
            if let Some(value) = self.stack_protector {
                // don't need to propagate stack-protector on MSVC since /GS is already the default
                // https://learn.microsoft.com/en-us/cpp/build/reference/gs-buffer-security-check?view=msvc-170
                //
                // Do NOT `stack-protector=none` since it weakens security for C code,
                // and `-Zstack-protector=basic` is deprecated and will be removed soon.
                let cc_flag = match value {
                    "strong" => Some("-fstack-protector-strong"),
                    "all" => Some("-fstack-protector-all"),
                    _ => None,
                };
                if let Some(cc_flag) = cc_flag {
                    push_if_supported(cc_flag.into());
                }
            }
            // Keep the order, so that a later `-feature` still overrides an earlier `+feature`.
            for feature in &self.target_features {
                if let Some(cc_flag) = Self::target_feature_cc_flag(target.arch, feature) {
                    push_if_supported(cc_flag.into());
                }
            }
        }

        // Compiler-exclusive flags
        match family {
            ToolFamily::Clang { .. } => {
                // Do NOT forward `-Cprofile-generate` / `-Cprofile-use` as
                // `-fprofile-generate` / `-fprofile-use`, even though rustc and clang both use
                // LLVM's instrumentation-based PGO. The profiling runtime (`profiler_builtins`)
                // and the raw/indexed profile formats are tied to the LLVM version, and the LLVM
                // behind the C compiler is frequently not the one behind rustc (e.g. Apple clang
                // vs. the LLVM bundled with rustc). C objects instrumented by one toolchain crash
                // at run time when linked against the other toolchain's profiling runtime, and a
                // `.profdata` merged by one toolchain's `llvm-profdata` is rejected by the other.
                // Users whose C compiler matches rustc's LLVM can opt in by passing
                // `-fprofile-generate=...` / `-fprofile-use=...` via `CFLAGS` / `CXXFLAGS`.
                // See https://github.com/rust-lang/cc-rs/issues/1986.

                // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fembed-bitcode
                if let Some(value) = self.embed_bitcode {
                    let cc_val = if value { "all" } else { "off" };
                    push_if_supported(format!("-fembed-bitcode={cc_val}").into());
                }

                // https://doc.rust-lang.org/rustc/linker-plugin-lto.html
                if self.linker_plugin_lto.unwrap_or(false) {
                    // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-flto
                    // In order to use linker-plugin-lto to achieve cross-lang lto, cc has to use thin LTO
                    // to compile the c/c++ libraries because llvm linker plugin/lld uses thin LTO by default.
                    // And for thin LTO in linker plugin to work, the archive also has to be compiled using thin LTO,
                    // since thin LTO generates extra information that fat LTO does not generate that
                    // is required for thin LTO process.
                    push_if_supported("-flto=thin".into());
                }
                // https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-mguard
                if let Some(value) = self.control_flow_guard {
                    let cc_val = match value {
                        "y" | "yes" | "on" | "true" | "checks" => Some("cf"),
                        "nochecks" => Some("cf-nochecks"),
                        "n" | "no" | "off" | "false" => Some("none"),
                        _ => None,
                    };
                    if let Some(cc_val) = cc_val {
                        push_if_supported(format!("-mguard={cc_val}").into());
                    }
                }
            }
            ToolFamily::Gnu => {}
            ToolFamily::Msvc { .. } => {
                // https://learn.microsoft.com/en-us/cpp/build/reference/guard-enable-control-flow-guard
                if let Some(value) = self.control_flow_guard {
                    let cc_val = match value {
                        "y" | "yes" | "on" | "true" | "checks" => Some("cf"),
                        "n" | "no" | "off" | "false" => Some("cf-"),
                        _ => None,
                    };
                    if let Some(cc_val) = cc_val {
                        push_if_supported(format!("/guard:{cc_val}").into());
                    }
                }
                // https://learn.microsoft.com/en-us/cpp/build/reference/oy-frame-pointer-omission
                if let Some(value) = self.force_frame_pointers {
                    // Flag is unsupported on 64-bit arches
                    if !target.arch.contains("64") {
                        let cc_flag = if value { "/Oy-" } else { "/Oy" };
                        push_if_supported(cc_flag.into());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/cc/unit/src/flags/tests_425.rs"));
