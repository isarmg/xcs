#[cfg(test)]
mod tests {
    use core::num::NonZeroUsize;

    use crate::util::testutil::*;

    #[cfg(__ZEROCOPY_INTERNAL_USE_ONLY_NIGHTLY_FEATURES_IN_TESTS)]
    mod nightly {
        use super::super::*;
        use crate::util::testutil::*;

        // FIXME(#29), FIXME(https://github.com/rust-lang/rust/issues/69835):
        // Remove this `cfg` when `size_of_val_raw` is stabilized.
        #[allow(clippy::decimal_literal_representation)]
        #[test]
        fn test_trailing_field_offset() {
            assert_eq!(mem::align_of::<Aligned64kAllocation>(), _64K);

            macro_rules! test {
                (#[$cfg:meta] ($($ts:ty),* ; $trailing_field_ty:ty) => $expect:expr) => {{
                    #[$cfg]
                    struct Test($(#[allow(dead_code)] $ts,)* #[allow(dead_code)] $trailing_field_ty);
                    assert_eq!(test!(@offset $($ts),* ; $trailing_field_ty), $expect);
                }};
                (#[$cfg:meta] $(#[$cfgs:meta])* ($($ts:ty),* ; $trailing_field_ty:ty) => $expect:expr) => {
                    test!(#[$cfg] ($($ts),* ; $trailing_field_ty) => $expect);
                    test!($(#[$cfgs])* ($($ts),* ; $trailing_field_ty) => $expect);
                };
                (@offset ; $_trailing:ty) => { trailing_field_offset!(Test, 0) };
                (@offset $_t:ty ; $_trailing:ty) => { trailing_field_offset!(Test, 1) };
            }

            test!(#[repr(C)] #[repr(transparent)] #[repr(packed)](; u8) => Some(0));
            test!(#[repr(C)] #[repr(transparent)] #[repr(packed)](; [u8]) => Some(0));
            test!(#[repr(C)] #[repr(C, packed)] (u8; u8) => Some(1));
            test!(#[repr(C)] (; AU64) => Some(0));
            test!(#[repr(C)] (; [AU64]) => Some(0));
            test!(#[repr(C)] (u8; AU64) => Some(8));
            test!(#[repr(C)] (u8; [AU64]) => Some(8));

            #[derive(
                Immutable, FromBytes, Eq, PartialEq, Ord, PartialOrd, Default, Debug, Copy, Clone,
            )]
            #[repr(C)]
            pub(crate) struct Nested<T, U: ?Sized> {
                _t: T,
                _u: U,
            }

            test!(#[repr(C)] (; Nested<u8, AU64>) => Some(0));
            test!(#[repr(C)] (; Nested<u8, [AU64]>) => Some(0));
            test!(#[repr(C)] (u8; Nested<u8, AU64>) => Some(8));
            test!(#[repr(C)] (u8; Nested<u8, [AU64]>) => Some(8));

            // Test that `packed(N)` limits the offset of the trailing field.
            test!(#[repr(C, packed(        1))] (u8; elain::Align<        2>) => Some(        1));
            test!(#[repr(C, packed(        2))] (u8; elain::Align<        4>) => Some(        2));
            test!(#[repr(C, packed(        4))] (u8; elain::Align<        8>) => Some(        4));
            test!(#[repr(C, packed(        8))] (u8; elain::Align<       16>) => Some(        8));
            test!(#[repr(C, packed(       16))] (u8; elain::Align<       32>) => Some(       16));
            test!(#[repr(C, packed(       32))] (u8; elain::Align<       64>) => Some(       32));
            test!(#[repr(C, packed(       64))] (u8; elain::Align<      128>) => Some(       64));
            test!(#[repr(C, packed(      128))] (u8; elain::Align<      256>) => Some(      128));
            test!(#[repr(C, packed(      256))] (u8; elain::Align<      512>) => Some(      256));
            test!(#[repr(C, packed(      512))] (u8; elain::Align<     1024>) => Some(      512));
            test!(#[repr(C, packed(     1024))] (u8; elain::Align<     2048>) => Some(     1024));
            test!(#[repr(C, packed(     2048))] (u8; elain::Align<     4096>) => Some(     2048));
            test!(#[repr(C, packed(     4096))] (u8; elain::Align<     8192>) => Some(     4096));
            test!(#[repr(C, packed(     8192))] (u8; elain::Align<    16384>) => Some(     8192));
            test!(#[repr(C, packed(    16384))] (u8; elain::Align<    32768>) => Some(    16384));
            test!(#[repr(C, packed(    32768))] (u8; elain::Align<    65536>) => Some(    32768));
            test!(#[repr(C, packed(    65536))] (u8; elain::Align<   131072>) => Some(    65536));
            /* Alignments above 65536 are not yet supported.
            test!(#[repr(C, packed(   131072))] (u8; elain::Align<   262144>) => Some(   131072));
            test!(#[repr(C, packed(   262144))] (u8; elain::Align<   524288>) => Some(   262144));
            test!(#[repr(C, packed(   524288))] (u8; elain::Align<  1048576>) => Some(   524288));
            test!(#[repr(C, packed(  1048576))] (u8; elain::Align<  2097152>) => Some(  1048576));
            test!(#[repr(C, packed(  2097152))] (u8; elain::Align<  4194304>) => Some(  2097152));
            test!(#[repr(C, packed(  4194304))] (u8; elain::Align<  8388608>) => Some(  4194304));
            test!(#[repr(C, packed(  8388608))] (u8; elain::Align< 16777216>) => Some(  8388608));
            test!(#[repr(C, packed( 16777216))] (u8; elain::Align< 33554432>) => Some( 16777216));
            test!(#[repr(C, packed( 33554432))] (u8; elain::Align< 67108864>) => Some( 33554432));
            test!(#[repr(C, packed( 67108864))] (u8; elain::Align< 33554432>) => Some( 67108864));
            test!(#[repr(C, packed( 33554432))] (u8; elain::Align<134217728>) => Some( 33554432));
            test!(#[repr(C, packed(134217728))] (u8; elain::Align<268435456>) => Some(134217728));
            test!(#[repr(C, packed(268435456))] (u8; elain::Align<268435456>) => Some(268435456));
            */

            // Test that `align(N)` does not limit the offset of the trailing field.
            test!(#[repr(C, align(        1))] (u8; elain::Align<        2>) => Some(        2));
            test!(#[repr(C, align(        2))] (u8; elain::Align<        4>) => Some(        4));
            test!(#[repr(C, align(        4))] (u8; elain::Align<        8>) => Some(        8));
            test!(#[repr(C, align(        8))] (u8; elain::Align<       16>) => Some(       16));
            test!(#[repr(C, align(       16))] (u8; elain::Align<       32>) => Some(       32));
            test!(#[repr(C, align(       32))] (u8; elain::Align<       64>) => Some(       64));
            test!(#[repr(C, align(       64))] (u8; elain::Align<      128>) => Some(      128));
            test!(#[repr(C, align(      128))] (u8; elain::Align<      256>) => Some(      256));
            test!(#[repr(C, align(      256))] (u8; elain::Align<      512>) => Some(      512));
            test!(#[repr(C, align(      512))] (u8; elain::Align<     1024>) => Some(     1024));
            test!(#[repr(C, align(     1024))] (u8; elain::Align<     2048>) => Some(     2048));
            test!(#[repr(C, align(     2048))] (u8; elain::Align<     4096>) => Some(     4096));
            test!(#[repr(C, align(     4096))] (u8; elain::Align<     8192>) => Some(     8192));
            test!(#[repr(C, align(     8192))] (u8; elain::Align<    16384>) => Some(    16384));
            test!(#[repr(C, align(    16384))] (u8; elain::Align<    32768>) => Some(    32768));
            test!(#[repr(C, align(    32768))] (u8; elain::Align<    65536>) => Some(    65536));
            /* Alignments above 65536 are not yet supported.
            test!(#[repr(C, align(    65536))] (u8; elain::Align<   131072>) => Some(   131072));
            test!(#[repr(C, align(   131072))] (u8; elain::Align<   262144>) => Some(   262144));
            test!(#[repr(C, align(   262144))] (u8; elain::Align<   524288>) => Some(   524288));
            test!(#[repr(C, align(   524288))] (u8; elain::Align<  1048576>) => Some(  1048576));
            test!(#[repr(C, align(  1048576))] (u8; elain::Align<  2097152>) => Some(  2097152));
            test!(#[repr(C, align(  2097152))] (u8; elain::Align<  4194304>) => Some(  4194304));
            test!(#[repr(C, align(  4194304))] (u8; elain::Align<  8388608>) => Some(  8388608));
            test!(#[repr(C, align(  8388608))] (u8; elain::Align< 16777216>) => Some( 16777216));
            test!(#[repr(C, align( 16777216))] (u8; elain::Align< 33554432>) => Some( 33554432));
            test!(#[repr(C, align( 33554432))] (u8; elain::Align< 67108864>) => Some( 67108864));
            test!(#[repr(C, align( 67108864))] (u8; elain::Align< 33554432>) => Some( 33554432));
            test!(#[repr(C, align( 33554432))] (u8; elain::Align<134217728>) => Some(134217728));
            test!(#[repr(C, align(134217728))] (u8; elain::Align<268435456>) => Some(268435456));
            */
        }

        // FIXME(#29), FIXME(https://github.com/rust-lang/rust/issues/69835):
        // Remove this `cfg` when `size_of_val_raw` is stabilized.
        #[allow(clippy::decimal_literal_representation)]
        #[test]
        fn test_align_of_dst() {
            // Test that `align_of!` correctly computes the alignment of DSTs.
            assert_eq!(align_of!([elain::Align<1>]), Some(1));
            assert_eq!(align_of!([elain::Align<2>]), Some(2));
            assert_eq!(align_of!([elain::Align<4>]), Some(4));
            assert_eq!(align_of!([elain::Align<8>]), Some(8));
            assert_eq!(align_of!([elain::Align<16>]), Some(16));
            assert_eq!(align_of!([elain::Align<32>]), Some(32));
            assert_eq!(align_of!([elain::Align<64>]), Some(64));
            assert_eq!(align_of!([elain::Align<128>]), Some(128));
            assert_eq!(align_of!([elain::Align<256>]), Some(256));
            assert_eq!(align_of!([elain::Align<512>]), Some(512));
            assert_eq!(align_of!([elain::Align<1024>]), Some(1024));
            assert_eq!(align_of!([elain::Align<2048>]), Some(2048));
            assert_eq!(align_of!([elain::Align<4096>]), Some(4096));
            assert_eq!(align_of!([elain::Align<8192>]), Some(8192));
            assert_eq!(align_of!([elain::Align<16384>]), Some(16384));
            assert_eq!(align_of!([elain::Align<32768>]), Some(32768));
            assert_eq!(align_of!([elain::Align<65536>]), Some(65536));
            /* Alignments above 65536 are not yet supported.
            assert_eq!(align_of!([elain::Align<131072>]), Some(131072));
            assert_eq!(align_of!([elain::Align<262144>]), Some(262144));
            assert_eq!(align_of!([elain::Align<524288>]), Some(524288));
            assert_eq!(align_of!([elain::Align<1048576>]), Some(1048576));
            assert_eq!(align_of!([elain::Align<2097152>]), Some(2097152));
            assert_eq!(align_of!([elain::Align<4194304>]), Some(4194304));
            assert_eq!(align_of!([elain::Align<8388608>]), Some(8388608));
            assert_eq!(align_of!([elain::Align<16777216>]), Some(16777216));
            assert_eq!(align_of!([elain::Align<33554432>]), Some(33554432));
            assert_eq!(align_of!([elain::Align<67108864>]), Some(67108864));
            assert_eq!(align_of!([elain::Align<33554432>]), Some(33554432));
            assert_eq!(align_of!([elain::Align<134217728>]), Some(134217728));
            assert_eq!(align_of!([elain::Align<268435456>]), Some(268435456));
            */
        }
    }

    #[test]
    fn test_enum_casts() {
        // Test that casting the variants of enums with signed integer reprs to
        // unsigned integers obeys expected signed -> unsigned casting rules.

        #[repr(i8)]
        enum ReprI8 {
            MinusOne = -1,
            Zero = 0,
            Min = i8::MIN,
            Max = i8::MAX,
        }

        #[allow(clippy::as_conversions)]
        let x = ReprI8::MinusOne as u8;
        assert_eq!(x, u8::MAX);

        #[allow(clippy::as_conversions)]
        let x = ReprI8::Zero as u8;
        assert_eq!(x, 0);

        #[allow(clippy::as_conversions)]
        let x = ReprI8::Min as u8;
        assert_eq!(x, 128);

        #[allow(clippy::as_conversions)]
        let x = ReprI8::Max as u8;
        assert_eq!(x, 127);
    }

    #[test]
    fn test_struct_padding() {
        // Test that, for each provided repr, `struct_padding!` reports the
        // expected value.
        macro_rules! test {
            (#[$cfg:meta] ($($ts:ty),*) => $expect:expr) => {{
                #[$cfg]
                #[allow(dead_code)]
                struct Test($($ts),*);
                assert_eq!(struct_padding!(Test, None::<NonZeroUsize>, None::<NonZeroUsize>, [$($ts),*]), $expect);
            }};
            (#[$cfg:meta] $(#[$cfgs:meta])* ($($ts:ty),*) => $expect:expr) => {
                test!(#[$cfg] ($($ts),*) => $expect);
                test!($(#[$cfgs])* ($($ts),*) => $expect);
            };
        }

        test!(#[repr(C)] #[repr(transparent)] #[repr(packed)] () => 0);
        test!(#[repr(C)] #[repr(transparent)] #[repr(packed)] (u8) => 0);
        test!(#[repr(C)] #[repr(transparent)] #[repr(packed)] (u8, ()) => 0);
        test!(#[repr(C)] #[repr(packed)] (u8, u8) => 0);

        test!(#[repr(C)] (u8, AU64) => 7);
        // Rust won't let you put `#[repr(packed)]` on a type which contains a
        // `#[repr(align(n > 1))]` type (`AU64`), so we have to use `u64` here.
        // It's not ideal, but it definitely has align > 1 on /some/ of our CI
        // targets, and this isn't a particularly complex macro we're testing
        // anyway.
        test!(#[repr(packed)] (u8, u64) => 0);
    }

    #[test]
    fn test_repr_c_struct_padding() {
        // Test that, for each provided repr, `repr_c_struct_padding!` reports
        // the expected value.
        macro_rules! test {
            (($($ts:tt),*) => $expect:expr) => {{
                #[repr(C)]
                #[allow(dead_code)]
                struct Test($($ts),*);
                assert_eq!(repr_c_struct_has_padding!(Test, None::<NonZeroUsize>, None::<NonZeroUsize>, [$($ts),*]), $expect);
            }};
        }

        // Test static padding
        test!(() => false);
        test!(([u8]) => false);
        test!((u8) => false);
        test!((u8, [u8]) => false);
        test!((u8, ()) => false);
        test!((u8, (), [u8]) => false);
        test!((u8, u8) => false);
        test!((u8, u8, [u8]) => false);

        test!((u8, AU64) => true);
        test!((u8, AU64, [u8]) => true);

        // Test dynamic padding
        test!((AU64, [AU64]) => false);
        test!((u8, [AU64]) => true);

        #[repr(align(4))]
        struct AU32(#[allow(unused)] u32);
        test!((AU64, [AU64]) => false);
        test!((AU64, [AU32]) => true);
    }

    #[test]
    fn test_union_padding() {
        // Test that, for each provided repr, `union_padding!` reports the
        // expected value.
        macro_rules! test {
            (#[$cfg:meta] {$($fs:ident: $ts:ty),*} => $expect:expr) => {{
                #[$cfg]
                #[allow(unused)] // fields are never read
                union Test{ $($fs: $ts),* }
                assert_eq!(union_padding!(Test, None::<NonZeroUsize>, None::<usize>, [$($ts),*]), $expect);
            }};
            (#[$cfg:meta] $(#[$cfgs:meta])* {$($fs:ident: $ts:ty),*} => $expect:expr) => {
                test!(#[$cfg] {$($fs: $ts),*} => $expect);
                test!($(#[$cfgs])* {$($fs: $ts),*} => $expect);
            };
        }

        test!(#[repr(C)] #[repr(packed)] {a: u8} => 0);
        test!(#[repr(C)] #[repr(packed)] {a: u8, b: u8} => 0);

        // Rust won't let you put `#[repr(packed)]` on a type which contains a
        // `#[repr(align(n > 1))]` type (`AU64`), so we have to use `u64` here.
        // It's not ideal, but it definitely has align > 1 on /some/ of our CI
        // targets, and this isn't a particularly complex macro we're testing
        // anyway.
        test!(#[repr(C)] #[repr(packed)] {a: u8, b: u64} => 7);
    }

    #[test]
    fn test_enum_padding() {
        // Test that, for each provided repr, `enum_has_padding!` reports the
        // expected value.
        macro_rules! test {
            (#[repr($disc:ident $(, $c:ident)?)] { $($vs:ident ($($ts:ty),*),)* } => $expect:expr) => {
                test!(@case #[repr($disc $(, $c)?)] { $($vs ($($ts),*),)* } => $expect);
            };
            (#[repr($disc:ident $(, $c:ident)?)] #[$cfg:meta] $(#[$cfgs:meta])* { $($vs:ident ($($ts:ty),*),)* } => $expect:expr) => {
                test!(@case #[repr($disc $(, $c)?)] #[$cfg] { $($vs ($($ts),*),)* } => $expect);
                test!(#[repr($disc $(, $c)?)] $(#[$cfgs])* { $($vs ($($ts),*),)* } => $expect);
            };
            (@case #[repr($disc:ident $(, $c:ident)?)] $(#[$cfg:meta])? { $($vs:ident ($($ts:ty),*),)* } => $expect:expr) => {{
                #[repr($disc $(, $c)?)]
                $(#[$cfg])?
                #[allow(unused)] // variants and fields are never used
                enum Test {
                    $($vs ($($ts),*),)*
                }
                assert_eq!(
                    enum_padding!(Test, None::<NonZeroUsize>, None::<NonZeroUsize>, $disc, $([$($ts),*]),*),
                    $expect
                );
            }};
        }

        #[allow(unused)]
        #[repr(align(2))]
        struct U16(u16);

        #[allow(unused)]
        #[repr(align(4))]
        struct U32(u32);

        test!(#[repr(u8)] #[repr(C)] {
            A(u8),
        } => 0);
        test!(#[repr(u16)] #[repr(C)] {
            A(u8, u8),
            B(U16),
        } => 0);
        test!(#[repr(u32)] #[repr(C)] {
            A(u8, u8, u8, u8),
            B(U16, u8, u8),
            C(u8, u8, U16),
            D(U16, U16),
            E(U32),
        } => 0);

        // `repr(int)` can pack the discriminant more efficiently
        test!(#[repr(u8)] {
            A(u8, U16),
        } => 0);
        test!(#[repr(u8)] {
            A(u8, U16, U32),
        } => 0);

        // `repr(C)` cannot
        test!(#[repr(u8, C)] {
            A(u8, U16),
        } => 2);
        test!(#[repr(u8, C)] {
            A(u8, u8, u8, U32),
        } => 4);

        // And field ordering can always cause problems
        test!(#[repr(u8)] #[repr(C)] {
            A(U16, u8),
        } => 2);
        test!(#[repr(u8)] #[repr(C)] {
            A(U32, u8, u8, u8),
        } => 4);
    }
}
