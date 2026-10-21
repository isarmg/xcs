#[cfg(any(test, kani))]
mod tests {
    use super::*;

    #[cfg(not(kani))]
    mod compatibility {
        pub(super) use rand::{
            distributions::{Distribution, Standard},
            rngs::SmallRng,
            Rng, SeedableRng,
        };

        pub(crate) trait Arbitrary {}

        impl<T> Arbitrary for T {}
    }

    #[cfg(kani)]
    mod compatibility {
        pub(crate) use kani::Arbitrary;

        pub(crate) struct SmallRng;

        impl SmallRng {
            pub(crate) fn seed_from_u64(_state: u64) -> Self {
                Self
            }
        }

        pub(crate) trait Rng {
            fn sample<T, D: Distribution<T>>(&mut self, _distr: D) -> T
            where
                T: Arbitrary,
            {
                kani::any()
            }
        }

        impl Rng for SmallRng {}

        pub(crate) trait Distribution<T> {}
        impl<T, U> Distribution<T> for U {}

        pub(crate) struct Standard;
    }

    use compatibility::*;

    // A native integer type (u16, i32, etc).
    trait Native: Arbitrary + FromBytes + IntoBytes + Immutable + Copy + PartialEq + Debug {
        const ZERO: Self;
        const MAX_VALUE: Self;

        type Distribution: Distribution<Self>;
        const DIST: Self::Distribution;

        fn rand<R: Rng>(rng: &mut R) -> Self {
            rng.sample(Self::DIST)
        }

        #[cfg_attr(kani, allow(unused))]
        fn checked_add(self, rhs: Self) -> Option<Self>;

        #[cfg_attr(kani, allow(unused))]
        fn checked_div(self, rhs: Self) -> Option<Self>;

        #[cfg_attr(kani, allow(unused))]
        fn checked_mul(self, rhs: Self) -> Option<Self>;

        #[cfg_attr(kani, allow(unused))]
        fn checked_rem(self, rhs: Self) -> Option<Self>;

        #[cfg_attr(kani, allow(unused))]
        fn checked_sub(self, rhs: Self) -> Option<Self>;

        #[cfg_attr(kani, allow(unused))]
        fn checked_shl(self, rhs: Self) -> Option<Self>;

        #[cfg_attr(kani, allow(unused))]
        fn checked_shr(self, rhs: Self) -> Option<Self>;

        fn is_nan(self) -> bool;

        /// For `f32` and `f64`, NaN values are not considered equal to
        /// themselves. This method is like `assert_eq!`, but it treats NaN
        /// values as equal.
        fn assert_eq_or_nan(self, other: Self) {
            let slf = (!self.is_nan()).then(|| self);
            let other = (!other.is_nan()).then(|| other);
            assert_eq!(slf, other);
        }
    }

    trait ByteArray:
        FromBytes + IntoBytes + Immutable + Copy + AsRef<[u8]> + AsMut<[u8]> + Debug + Default + Eq
    {
        /// Invert the order of the bytes in the array.
        fn invert(self) -> Self;
    }

    trait ByteOrderType:
        FromBytes + IntoBytes + Unaligned + Copy + Eq + Debug + Hash + From<Self::Native>
    {
        type Native: Native;
        type ByteArray: ByteArray;

        const ZERO: Self;

        fn new(native: Self::Native) -> Self;
        fn get(self) -> Self::Native;
        fn set(&mut self, native: Self::Native);
        fn from_bytes(bytes: Self::ByteArray) -> Self;
        fn into_bytes(self) -> Self::ByteArray;

        /// For `f32` and `f64`, NaN values are not considered equal to
        /// themselves. This method is like `assert_eq!`, but it treats NaN
        /// values as equal.
        fn assert_eq_or_nan(self, other: Self) {
            let slf = (!self.get().is_nan()).then(|| self);
            let other = (!other.get().is_nan()).then(|| other);
            assert_eq!(slf, other);
        }
    }

    trait ByteOrderTypeUnsigned: ByteOrderType {
        const MAX_VALUE: Self;
    }

    macro_rules! impl_byte_array {
        ($bytes:expr) => {
            impl ByteArray for [u8; $bytes] {
                fn invert(mut self) -> [u8; $bytes] {
                    self.reverse();
                    self
                }
            }
        };
    }

    impl_byte_array!(2);
    impl_byte_array!(4);
    impl_byte_array!(8);
    impl_byte_array!(16);

    macro_rules! impl_byte_order_type_unsigned {
        ($name:ident, unsigned) => {
            impl<O: ByteOrder> ByteOrderTypeUnsigned for $name<O> {
                const MAX_VALUE: $name<O> = $name::MAX_VALUE;
            }
        };
        ($name:ident, signed) => {};
    }

    macro_rules! impl_traits {
        ($name:ident, $native:ident, $sign:ident $(, @$float:ident)?) => {
            impl Native for $native {
                // For some types, `0 as $native` is required (for example, when
                // `$native` is a floating-point type; `0` is an integer), but
                // for other types, it's a trivial cast. In all cases, Clippy
                // thinks it's dangerous.
                #[allow(trivial_numeric_casts, clippy::as_conversions)]
                const ZERO: $native = 0 as $native;
                const MAX_VALUE: $native = $native::MAX;

                type Distribution = Standard;
                const DIST: Standard = Standard;

                impl_traits!(@float_dependent_methods $(@$float)?);
            }

            impl<O: ByteOrder> ByteOrderType for $name<O> {
                type Native = $native;
                type ByteArray = [u8; mem::size_of::<$native>()];

                const ZERO: $name<O> = $name::ZERO;

                fn new(native: $native) -> $name<O> {
                    $name::new(native)
                }

                fn get(self) -> $native {
                    $name::get(self)
                }

                fn set(&mut self, native: $native) {
                    $name::set(self, native)
                }

                fn from_bytes(bytes: [u8; mem::size_of::<$native>()]) -> $name<O> {
                    $name::from(bytes)
                }

                fn into_bytes(self) -> [u8; mem::size_of::<$native>()] {
                    <[u8; mem::size_of::<$native>()]>::from(self)
                }
            }

            impl_byte_order_type_unsigned!($name, $sign);
        };
        (@float_dependent_methods) => {
            fn checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
            fn checked_div(self, rhs: Self) -> Option<Self> { self.checked_div(rhs) }
            fn checked_mul(self, rhs: Self) -> Option<Self> { self.checked_mul(rhs) }
            fn checked_rem(self, rhs: Self) -> Option<Self> { self.checked_rem(rhs) }
            fn checked_sub(self, rhs: Self) -> Option<Self> { self.checked_sub(rhs) }
            fn checked_shl(self, rhs: Self) -> Option<Self> { self.checked_shl(rhs.try_into().unwrap_or(u32::MAX)) }
            fn checked_shr(self, rhs: Self) -> Option<Self> { self.checked_shr(rhs.try_into().unwrap_or(u32::MAX)) }
            fn is_nan(self) -> bool { false }
        };
        (@float_dependent_methods @float) => {
            fn checked_add(self, rhs: Self) -> Option<Self> { Some(self + rhs) }
            fn checked_div(self, rhs: Self) -> Option<Self> { Some(self / rhs) }
            fn checked_mul(self, rhs: Self) -> Option<Self> { Some(self * rhs) }
            fn checked_rem(self, rhs: Self) -> Option<Self> { Some(self % rhs) }
            fn checked_sub(self, rhs: Self) -> Option<Self> { Some(self - rhs) }
            fn checked_shl(self, _rhs: Self) -> Option<Self> { unimplemented!() }
            fn checked_shr(self, _rhs: Self) -> Option<Self> { unimplemented!() }
            fn is_nan(self) -> bool { self.is_nan() }
        };
    }

    impl_traits!(U16, u16, unsigned);
    impl_traits!(U32, u32, unsigned);
    impl_traits!(U64, u64, unsigned);
    impl_traits!(U128, u128, unsigned);
    impl_traits!(Usize, usize, unsigned);
    impl_traits!(I16, i16, signed);
    impl_traits!(I32, i32, signed);
    impl_traits!(I64, i64, signed);
    impl_traits!(I128, i128, signed);
    impl_traits!(Isize, isize, signed);
    impl_traits!(F32, f32, signed, @float);
    impl_traits!(F64, f64, signed, @float);

    macro_rules! call_for_unsigned_types {
        ($fn:ident, $byteorder:ident) => {
            $fn::<U16<$byteorder>>();
            $fn::<U32<$byteorder>>();
            $fn::<U64<$byteorder>>();
            $fn::<U128<$byteorder>>();
            $fn::<Usize<$byteorder>>();
        };
    }

    macro_rules! call_for_signed_types {
        ($fn:ident, $byteorder:ident) => {
            $fn::<I16<$byteorder>>();
            $fn::<I32<$byteorder>>();
            $fn::<I64<$byteorder>>();
            $fn::<I128<$byteorder>>();
            $fn::<Isize<$byteorder>>();
        };
    }

    macro_rules! call_for_float_types {
        ($fn:ident, $byteorder:ident) => {
            $fn::<F32<$byteorder>>();
            $fn::<F64<$byteorder>>();
        };
    }

    macro_rules! call_for_all_types {
        ($fn:ident, $byteorder:ident) => {
            call_for_unsigned_types!($fn, $byteorder);
            call_for_signed_types!($fn, $byteorder);
            call_for_float_types!($fn, $byteorder);
        };
    }

    #[cfg(target_endian = "big")]
    type NonNativeEndian = LittleEndian;
    #[cfg(target_endian = "little")]
    type NonNativeEndian = BigEndian;

    // We use a `u64` seed so that we can use `SeedableRng::seed_from_u64`.
    // `SmallRng`'s `SeedableRng::Seed` differs by platform, so if we wanted to
    // call `SeedableRng::from_seed`, which takes a `Seed`, we would need
    // conditional compilation by `target_pointer_width`.
    const RNG_SEED: u64 = 0x7A03CAE2F32B5B8F;

    const RAND_ITERS: usize = if cfg!(any(miri, kani)) {
        // The tests below which use this constant used to take a very long time
        // on Miri, which slows down local development and CI jobs. We're not
        // using Miri to check for the correctness of our code, but rather its
        // soundness, and at least in the context of these particular tests, a
        // single loop iteration is just as good for surfacing UB as multiple
        // iterations are.
        //
        // As of the writing of this comment, here's one set of measurements:
        //
        //   $ # RAND_ITERS == 1
        //   $ cargo miri test -- -Z unstable-options --report-time endian
        //   test byteorder::tests::test_native_endian ... ok <0.049s>
        //   test byteorder::tests::test_non_native_endian ... ok <0.061s>
        //
        //   $ # RAND_ITERS == 1024
        //   $ cargo miri test -- -Z unstable-options --report-time endian
        //   test byteorder::tests::test_native_endian ... ok <25.716s>
        //   test byteorder::tests::test_non_native_endian ... ok <38.127s>
        1
    } else {
        1024
    };

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/functions/src/byteorder/test_const_methods_1313.rs"));

    #[cfg_attr(test, test)]
    #[cfg_attr(kani, kani::proof)]
    fn test_zero() {
        fn test_zero<T: ByteOrderType>() {
            assert_eq!(T::ZERO.get(), T::Native::ZERO);
        }

        call_for_all_types!(test_zero, NativeEndian);
        call_for_all_types!(test_zero, NonNativeEndian);
    }

    #[cfg_attr(test, test)]
    #[cfg_attr(kani, kani::proof)]
    fn test_max_value() {
        fn test_max_value<T: ByteOrderTypeUnsigned>() {
            assert_eq!(T::MAX_VALUE.get(), T::Native::MAX_VALUE);
        }

        call_for_unsigned_types!(test_max_value, NativeEndian);
        call_for_unsigned_types!(test_max_value, NonNativeEndian);
    }

    #[cfg_attr(test, test)]
    #[cfg_attr(kani, kani::proof)]
    fn test_endian() {
        fn test<T: ByteOrderType>(invert: bool) {
            let mut r = SmallRng::seed_from_u64(RNG_SEED);
            for _ in 0..RAND_ITERS {
                let native = T::Native::rand(&mut r);
                let mut bytes = T::ByteArray::default();
                bytes.as_mut_bytes().copy_from_slice(native.as_bytes());
                if invert {
                    bytes = bytes.invert();
                }
                let mut from_native = T::new(native);
                let from_bytes = T::from_bytes(bytes);

                from_native.assert_eq_or_nan(from_bytes);
                from_native.get().assert_eq_or_nan(native);
                from_bytes.get().assert_eq_or_nan(native);

                assert_eq!(from_native.into_bytes(), bytes);
                assert_eq!(from_bytes.into_bytes(), bytes);

                let updated = T::Native::rand(&mut r);
                from_native.set(updated);
                from_native.get().assert_eq_or_nan(updated);
            }
        }

        fn test_native<T: ByteOrderType>() {
            test::<T>(false);
        }

        fn test_non_native<T: ByteOrderType>() {
            test::<T>(true);
        }

        call_for_all_types!(test_native, NativeEndian);
        call_for_all_types!(test_non_native, NonNativeEndian);
    }

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/functions/src/byteorder/test_float_bit_patterns_1387.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/functions/src/byteorder/test_ops_impls_1381.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/functions/src/byteorder/test_debug_impl_1580.rs"));

    #[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/zerocopy/functions/src/byteorder/test_byteorder_traits_coverage_1589.rs"));
}
