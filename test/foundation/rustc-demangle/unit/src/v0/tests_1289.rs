#[cfg(test)]
mod tests {
    use std::panic;
    use std::prelude::v1::*;

    macro_rules! t {
        ($a:expr, $b:expr) => {{
            assert_eq!(format!("{}", ::demangle($a)), $b);
        }};
    }
    macro_rules! t_nohash {
        ($a:expr, $b:expr) => {{
            assert_eq!(format!("{:#}", ::demangle($a)), $b);
        }};
    }
    macro_rules! t_nohash_type {
        ($a:expr, $b:expr) => {
            t_nohash!(concat!("_RMC0", $a), concat!("<", $b, ">"))
        };
    }
    macro_rules! t_const {
        ($mangled:expr, $value:expr) => {
            t_nohash!(
                concat!("_RIC0K", $mangled, "E"),
                concat!("::<", $value, ">")
            )
        };
    }
    macro_rules! t_const_suffixed {
        ($mangled:expr, $value:expr, $value_ty_suffix:expr) => {{
            t_const!($mangled, $value);
            t!(
                concat!("_RIC0K", $mangled, "E"),
                concat!("::<", $value, $value_ty_suffix, ">")
            );
        }};
    }

    #[test]
    fn demangle_crate_with_leading_digit() {
        t_nohash!("_RNvC6_123foo3bar", "123foo::bar");
    }

    #[test]
    fn demangle_crate_with_zero_disambiguator() {
        t!("_RC4f128", "f128");
        t_nohash!("_RC4f128", "f128");
    }

    #[test]
    fn demangle_utf8_idents() {
        t_nohash!(
            "_RNqCs4fqI2P2rA04_11utf8_identsu30____7hkackfecea1cbdathfdh9hlq6y",
            "utf8_idents::საჭმელად_გემრიელი_სადილი"
        );
    }

    #[test]
    fn demangle_closure() {
        t_nohash!(
            "_RNCNCNgCs6DXkGYLi8lr_2cc5spawn00B5_",
            "cc::spawn::{closure#0}::{closure#0}"
        );
        t_nohash!(
            "_RNCINkXs25_NgCsbmNqQUJIY6D_4core5sliceINyB9_4IterhENuNgNoBb_4iter8iterator8Iterator9rpositionNCNgNpB9_6memchr7memrchrs_0E0Bb_",
            "<core::slice::Iter<u8> as core::iter::iterator::Iterator>::rposition::<core::slice::memchr::memrchr::{closure#1}>::{closure#0}"
        );
    }

    #[test]
    fn demangle_dyn_trait() {
        t_nohash!(
            "_RINbNbCskIICzLVDPPb_5alloc5alloc8box_freeDINbNiB4_5boxed5FnBoxuEp6OutputuEL_ECs1iopQbuBiw2_3std",
            "alloc::alloc::box_free::<dyn alloc::boxed::FnBox<(), Output = ()>>"
        );
    }

    #[test]
    fn demangle_dyn_trait_assoc_const_binding() {
        t_nohash_type!("DNtC5krate5Traitp1NKj0_EL_", "dyn krate::Trait<N = 0>");
    }

    #[test]
    fn demangle_pat_ty() {
        t_nohash_type!("WmRm1_m9_", "u32 is 1..=9");
        t_nohash_type!("WmORm1_m2_Rm5_m6_E", "u32 is 1..=2 | 5..=6");
        assert!(::v0::demangle("_RMC0WmORm1_m2_Rm5_m6_").is_err());
    }

    #[test]
    fn demangle_splat() {
        t_nohash!(
            "_RNvMNtCs5CcWRJzwAYz_4core6optionINtB2_6OptionFwTlEEuE6unwrapCsiksvdpJ6Jnj_14splat_mangling",
            "<core::option::Option<fn(#[splat] (i32,))>>::unwrap"
        );
        t_nohash!(
            "_RMNvCsiksvdpJ6Jnj_14splat_mangling4mainINtB0_4TypeFwThmEEuE",
            "<splat_mangling::main::Type<fn(#[splat] (u8, u32))>>"
        );
        t_nohash!(
            "_RMs0_NvCsiksvdpJ6Jnj_14splat_mangling4mainINtB3_4TypeFwTThmEEEuE",
            "<splat_mangling::main::Type<fn(#[splat] ((u8, u32),))>>"
        );
        t_nohash!(
            "_RMs2_NvCsiksvdpJ6Jnj_14splat_mangling4mainINtB3_4TypePFwTmaEEuE",
            "<splat_mangling::main::Type<*const fn(#[splat] (u32, i8))>>"
        );
        t_nohash!(
            "_RMs4_NvCsiksvdpJ6Jnj_14splat_mangling4mainINtB3_4TypeFwTmaEdEuE",
            "<splat_mangling::main::Type<fn(#[splat] (u32, i8), f64)>>"
        );
    }

    #[test]
    fn demangle_const_generics_preview() {
        // NOTE(eddyb) this was hand-written, before rustc had working
        // const generics support (but the mangling format did include them).
        t_nohash_type!(
            "INtC8arrayvec8ArrayVechKj7b_E",
            "arrayvec::ArrayVec<u8, 123>"
        );
        t_const_suffixed!("j7b_", "123", "usize");
    }

    #[test]
    fn demangle_min_const_generics() {
        t_const!("p", "_");
        t_const_suffixed!("hb_", "11", "u8");
        t_const_suffixed!("off00ff00ff00ff00ff_", "0xff00ff00ff00ff00ff", "u128");
        t_const_suffixed!("s98_", "152", "i16");
        t_const_suffixed!("anb_", "-11", "i8");
        t_const!("b0_", "false");
        t_const!("b1_", "true");
        t_const!("c76_", "'v'");
        t_const!("c22_", r#"'"'"#);
        t_const!("ca_", "'\\n'");
        t_const!("c2202_", "'∂'");
    }

    #[test]
    fn demangle_const_str() {
        t_const!("e616263_", "{*\"abc\"}");
        t_const!("e27_", r#"{*"'"}"#);
        t_const!("e090a_", "{*\"\\t\\n\"}");
        t_const!("ee28882c3bc_", "{*\"∂ü\"}");
        t_const!(
            "ee183a1e18390e183ade1839be18394e1839ae18390e183935fe18392e18394e1839b\
              e183a0e18398e18394e1839ae183985fe183a1e18390e18393e18398e1839ae18398_",
            "{*\"საჭმელად_გემრიელი_სადილი\"}"
        );
        t_const!(
            "ef09f908af09fa688f09fa686f09f90ae20c2a720f09f90b6f09f9192e298\
              95f09f94a520c2a720f09fa7a1f09f929bf09f929af09f9299f09f929c_",
            "{*\"🐊🦈🦆🐮 § 🐶👒☕🔥 § 🧡💛💚💙💜\"}"
        );
    }

    // NOTE(eddyb) this uses the same strings as `demangle_const_str` and should
    // be kept in sync with it - while a macro could be used to generate both
    // `str` and `&str` tests, from a single list of strings, this seems clearer.
    #[test]
    fn demangle_const_ref_str() {
        t_const!("Re616263_", "\"abc\"");
        t_const!("Re27_", r#""'""#);
        t_const!("Re090a_", "\"\\t\\n\"");
        t_const!("Ree28882c3bc_", "\"∂ü\"");
        t_const!(
            "Ree183a1e18390e183ade1839be18394e1839ae18390e183935fe18392e18394e1839b\
               e183a0e18398e18394e1839ae183985fe183a1e18390e18393e18398e1839ae18398_",
            "\"საჭმელად_გემრიელი_სადილი\""
        );
        t_const!(
            "Ref09f908af09fa688f09fa686f09f90ae20c2a720f09f90b6f09f9192e298\
               95f09f94a520c2a720f09fa7a1f09f929bf09f929af09f9299f09f929c_",
            "\"🐊🦈🦆🐮 § 🐶👒☕🔥 § 🧡💛💚💙💜\""
        );
    }

    #[test]
    fn demangle_const_ref() {
        t_const!("Rp", "{&_}");
        t_const!("Rh7b_", "{&123}");
        t_const!("Rb0_", "{&false}");
        t_const!("Rc58_", "{&'X'}");
        t_const!("RRRh0_", "{&&&0}");
        t_const!("RRRe_", "{&&\"\"}");
        t_const!("QAE", "{&mut []}");
    }

    #[test]
    fn demangle_const_array() {
        t_const!("AE", "{[]}");
        t_const!("Aj0_E", "{[0]}");
        t_const!("Ah1_h2_h3_E", "{[1, 2, 3]}");
        t_const!("ARe61_Re62_Re63_E", "{[\"a\", \"b\", \"c\"]}");
        t_const!("AAh1_h2_EAh3_h4_EE", "{[[1, 2], [3, 4]]}");
    }

    #[test]
    fn demangle_const_tuple() {
        t_const!("TE", "{()}");
        t_const!("Tj0_E", "{(0,)}");
        t_const!("Th1_b0_E", "{(1, false)}");
        t_const!(
            "TRe616263_c78_RAh1_h2_h3_EE",
            "{(\"abc\", 'x', &[1, 2, 3])}"
        );
    }

    #[test]
    fn demangle_const_adt() {
        t_const!(
            "VNvINtNtC4core6option6OptionjE4NoneU",
            "{core::option::Option::<usize>::None}"
        );
        t_const!(
            "VNvINtNtC4core6option6OptionjE4SomeTj0_E",
            "{core::option::Option::<usize>::Some(0)}"
        );
        t_const!(
            "VNtC3foo3BarS1sRe616263_2chc78_5sliceRAh1_h2_h3_EE",
            "{foo::Bar { s: \"abc\", ch: 'x', slice: &[1, 2, 3] }}"
        );
    }

    #[test]
    fn demangle_exponential_explosion() {
        // NOTE(eddyb) because of the prefix added by `t_nohash_type!` is
        // 3 bytes long, `B2_` refers to the start of the type, not `B_`.
        // 6 backrefs (`B8_E` through `B3_E`) result in 2^6 = 64 copies of `_`.
        // Also, because the `p` (`_`) type is after all of the starts of the
        // backrefs, it can be replaced with any other type, independently.
        t_nohash_type!(
            concat!("TTTTTT", "p", "B8_E", "B7_E", "B6_E", "B5_E", "B4_E", "B3_E"),
            "((((((_, _), (_, _)), ((_, _), (_, _))), (((_, _), (_, _)), ((_, _), (_, _)))), \
             ((((_, _), (_, _)), ((_, _), (_, _))), (((_, _), (_, _)), ((_, _), (_, _))))), \
             (((((_, _), (_, _)), ((_, _), (_, _))), (((_, _), (_, _)), ((_, _), (_, _)))), \
             ((((_, _), (_, _)), ((_, _), (_, _))), (((_, _), (_, _)), ((_, _), (_, _))))))"
        );
    }

    #[test]
    fn demangle_thinlto() {
        t_nohash!("_RC3foo.llvm.9D1C9369", "foo");
        t_nohash!("_RC3foo.llvm.9D1C9369@@16", "foo");
        t_nohash!("_RNvC9backtrace3foo.llvm.A5310EB9", "backtrace::foo");
    }

    #[test]
    fn demangle_extra_suffix() {
        // From alexcrichton/rustc-demangle#27:
        t_nohash!(
            "_RNvNtNtNtNtCs92dm3009vxr_4rand4rngs7adapter9reseeding4fork23FORK_HANDLER_REGISTERED.0.0",
            "rand::rngs::adapter::reseeding::fork::FORK_HANDLER_REGISTERED.0.0"
        );
    }

    #[test]
    fn demangling_limits() {
        // Stress tests found via fuzzing.

        for sym in include_str!("v0-large-test-symbols/early-recursion-limit")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            assert_eq!(
                super::demangle(sym).map(|_| ()),
                Err(super::ParseError::RecursedTooDeep)
            );
        }

        assert_contains!(
            ::demangle(
                "RIC20tRYIMYNRYFG05_EB5_B_B6_RRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR\
        RRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRB_E",
            )
            .to_string(),
            "{recursion limit reached}"
        );
    }

    #[test]
    fn recursion_limit_leaks() {
        // NOTE(eddyb) this test checks that both paths and types support the
        // recursion limit correctly, i.e. matching `push_depth` and `pop_depth`,
        // and don't leak "recursion levels" and trip the limit.
        // The test inputs are generated on the fly, using a repeated pattern,
        // as hardcoding the actual strings would be too verbose.
        // Also, `MAX_DEPTH` can be directly used, instead of assuming its value.
        for &(sym_leaf, expected_leaf) in &[("p", "_"), ("Rp", "&_"), ("C1x", "x")] {
            let mut sym = format!("_RIC0p");
            let mut expected = format!("::<_");
            for _ in 0..(super::MAX_DEPTH * 2) {
                sym.push_str(sym_leaf);
                expected.push_str(", ");
                expected.push_str(expected_leaf);
            }
            sym.push('E');
            expected.push('>');

            t_nohash!(&sym, expected);
        }
    }

    #[test]
    fn recursion_limit_backref_free_bypass() {
        // NOTE(eddyb) this test checks that long symbols cannot bypass the
        // recursion limit by not using backrefs, and cause a stack overflow.

        // This value was chosen to be high enough that stack overflows were
        // observed even with `cargo test --release`.
        let depth = 100_000;

        // In order to hide the long mangling from the initial "shallow" parse,
        // it's nested in an identifier (crate name), preceding its use.
        let mut sym = format!("_RIC{}", depth);
        let backref_start = sym.len() - 2;
        for _ in 0..depth {
            sym.push('R');
        }

        // Write a backref to just after the length of the identifier.
        sym.push('B');
        sym.push(char::from_digit((backref_start - 1) as u32, 36).unwrap());
        sym.push('_');

        // Close the `I` at the start.
        sym.push('E');

        assert_contains!(::demangle(&sym).to_string(), "{recursion limit reached}");
    }
}
