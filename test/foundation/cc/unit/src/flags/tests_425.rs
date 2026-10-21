#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn check(env: &str, expected: &RustcCodegenFlags) {
        let actual = RustcCodegenFlags::parse(env).unwrap();
        assert_eq!(actual, *expected);
    }

    #[test]
    fn codegen_type() {
        let expected = RustcCodegenFlags {
            code_model: Some("tiny"),
            ..RustcCodegenFlags::default()
        };
        check("-Ccode-model=tiny", &expected);
        check("-C\u{1f}code-model=tiny", &expected);
        check("--codegen\u{1f}code-model=tiny", &expected);
        check("--codegen=code-model=tiny", &expected);
    }

    #[test]
    fn precedence() {
        check(
            "-ccode-model=tiny\u{1f}-Ccode-model=small",
            &RustcCodegenFlags {
                code_model: Some("small"),
                ..RustcCodegenFlags::default()
            },
        );
    }

    #[test]
    fn two_valid_prefixes() {
        let expected = RustcCodegenFlags::default();
        check("-L\u{1f}-Clto", &expected);
    }

    #[test]
    fn stack_protector() {
        let expected = RustcCodegenFlags {
            stack_protector: Some("strong"),
            ..RustcCodegenFlags::default()
        };
        check("-Zstack-protector=strong", &expected);
        check("-Cstack-protector=strong", &expected);
    }

    #[test]
    fn three_valid_prefixes() {
        let expected = RustcCodegenFlags {
            lto: Some("true"),
            ..RustcCodegenFlags::default()
        };
        check("-L\u{1f}-L\u{1f}-Clto", &expected);
    }

    #[test]
    fn all_rustc_flags() {
        // Throw all possible flags at the parser to catch false positives
        let flags = [
            // Set all the flags we recognise first
            "-Ccode-model=tiny",
            "-Ccontrol-flow-guard=yes",
            "-Cembed-bitcode=no",
            "-Cforce-frame-pointers=yes",
            "-Clto=false",
            "-Clink-dead-code=yes",
            "-Cno-redzone=yes",
            "-Cno-vectorize-loops",
            "-Cno-vectorize-slp",
            "-Crelocation-model=pic",
            "-Csoft-float=yes",
            "-Zbranch-protection=bti,pac-ret,leaf",
            "-Cdwarf-version=5",
            "-Zstack-protector=strong",
            "-Ctarget-feature=+sve",
            // Set flags we don't recognise but rustc supports next
            // rustc flags
            "--cfg",
            "a",
            "--check-cfg 'cfg(verbose)",
            "-L",
            "/usr/lib/foo",
            "-l",
            "static:+whole-archive=mylib",
            "--crate-type=dylib",
            "--crate-name=foo",
            "--edition=2021",
            "--emit=asm",
            "--print=crate-name",
            "-g",
            "-O",
            "-o",
            "foooutput",
            "--out-dir",
            "foooutdir",
            "--target",
            "aarch64-unknown-linux-gnu",
            "-W",
            "missing-docs",
            "-D",
            "unused-variables",
            "--force-warn",
            "dead-code",
            "-A",
            "unused",
            "-F",
            "unused",
            "--cap-lints",
            "warn",
            "--version",
            "--verbose",
            "-v",
            "--extern",
            "foocrate",
            "--sysroot",
            "fooroot",
            "--error-format",
            "human",
            "--color",
            "auto",
            "--diagnostic-width",
            "80",
            "--remap-path-prefix",
            "foo=bar",
            "--json=artifact",
            // Codegen flags
            "-Car",
            "-Ccodegen-units=1",
            "-Ccollapse-macro-debuginfo=yes",
            "-Cdebug-assertions=yes",
            "-Cdebuginfo=1",
            "-Cdefault-linker-libraries=yes",
            "-Cdlltool=foo",
            "-Cextra-filename=foo",
            "-Cforce-unwind-tables=yes",
            "-Cincremental=foodir",
            "-Cinline-threshold=6",
            "-Cinstrument-coverage",
            "-Clink-arg=-foo",
            "-Clink-args=-foo",
            "-Clink-self-contained=yes",
            "-Clinker=lld",
            "-Clinker-flavor=ld.lld",
            "-Clinker-plugin-lto=/path",
            "-Cllvm-args=foo",
            "-Cmetadata=foo",
            "-Cno-prepopulate-passes",
            "-Cno-stack-check",
            "-Copt-level=3",
            "-Coverflow-checks=yes",
            "-Cpanic=abort",
            "-Cpasses=foopass",
            "-Cprefer-dynamic=yes",
            // Deliberately not inherited, see the note in `cc_flags`.
            "-Cprofile-generate=fooprofile",
            "-Cprofile-use=fooprofile",
            "-Crelro-level=partial",
            "-Cremark=all",
            "-Crpath=yes",
            "-Csave-temps=yes",
            "-Csplit-debuginfo=packed",
            "-Cstrip=symbols",
            "-Csymbol-mangling-version=v0",
            "-Ctarget-cpu=native",
            // Unstable options
            "-Ztune-cpu=machine",
        ];
        check(
            &flags.join("\u{1f}"),
            &RustcCodegenFlags {
                code_model: Some("tiny"),
                control_flow_guard: Some("yes"),
                embed_bitcode: Some(false),
                force_frame_pointers: Some(true),
                lto: Some("false"),
                no_redzone: Some(true),
                no_vectorize_loops: true,
                no_vectorize_slp: true,
                relocation_model: Some("pic"),
                soft_float: Some(true),
                branch_protection: Some("bti,pac-ret,leaf"),
                dwarf_version: Some(5),
                stack_protector: Some("strong"),
                linker_plugin_lto: Some(true),
                target_features: vec!["+sve"],
            },
        );
    }

    #[test]
    fn target_features() {
        let expected = RustcCodegenFlags {
            target_features: vec!["+avx2", "-sse4a"],
            ..RustcCodegenFlags::default()
        };
        check("-Ctarget-feature=+avx2,-sse4a", &expected);
        check("-C\u{1f}target-feature=+avx2,-sse4a", &expected);
        check("--codegen\u{1f}target-feature=+avx2,-sse4a", &expected);
        check("--codegen=target-feature=+avx2,-sse4a", &expected);
        // Repeated flags are all kept in order, so that later ones win.
        check(
            "-Ctarget-feature=+avx\u{1f}-Ctarget-feature=+avx2,-avx",
            &RustcCodegenFlags {
                target_features: vec!["+avx", "+avx2", "-avx"],
                ..RustcCodegenFlags::default()
            },
        );
    }

    #[test]
    fn target_feature_cc_flag() {
        let flag = RustcCodegenFlags::target_feature_cc_flag;
        assert_eq!(flag("x86_64", "+ssse3").as_deref(), Some("-mssse3"));
        assert_eq!(flag("x86_64", "-avx2").as_deref(), Some("-mno-avx2"));
        assert_eq!(flag("x86", "+sse4.2").as_deref(), Some("-msse4.2"));
        // Features that GCC and Clang name differently
        assert_eq!(flag("x86_64", "+bmi1").as_deref(), Some("-mbmi"));
        assert_eq!(flag("x86_64", "+pclmulqdq").as_deref(), Some("-mpclmul"));
        assert_eq!(flag("x86_64", "+rdrand").as_deref(), Some("-mrdrnd"));
        assert_eq!(flag("x86_64", "-cmpxchg16b").as_deref(), Some("-mno-cx16"));
        // Features without a known equivalent are skipped
        assert_eq!(flag("x86_64", "+crt-static"), None);
        assert_eq!(flag("x86_64", "+not-a-feature"), None);
        assert_eq!(flag("x86_64", "-sse2"), None);
        // rustc ignores features without a `+` or `-` prefix
        assert_eq!(flag("x86_64", "avx2"), None);
        // Other architectures are not mapped yet
        assert_eq!(flag("aarch64", "+neon"), None);
        assert_eq!(flag("aarch64", "+aes"), None);
    }
}
