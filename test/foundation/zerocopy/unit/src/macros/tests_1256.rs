#[cfg(test)]
mod tests {
    use crate::{
        byteorder::native_endian::{U16, U32},
        util::testutil::*,
        *,
    };

    #[derive(KnownLayout, Immutable, FromBytes, IntoBytes, PartialEq, Debug)]
    #[repr(C)]
    struct SliceDst<T, U> {
        a: T,
        b: [U],
    }

    #[test]
    fn test_transmute() {
        // Test that memory is transmuted as expected.
        let array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let x: [[u8; 2]; 4] = transmute!(array_of_u8s);
        assert_eq!(x, array_of_arrays);
        let x: [u8; 8] = transmute!(array_of_arrays);
        assert_eq!(x, array_of_u8s);

        // Test that memory is transmuted as expected when shrinking.
        let x: [[u8; 2]; 3] = transmute!(#![allow(shrink)] array_of_u8s);
        assert_eq!(x, [[0u8, 1], [2, 3], [4, 5]]);

        // Test that the source expression's value is forgotten rather than
        // dropped.
        #[derive(IntoBytes)]
        #[repr(transparent)]
        struct PanicOnDrop(());
        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic!("PanicOnDrop::drop");
            }
        }
        #[allow(clippy::let_unit_value)]
        let _: () = transmute!(PanicOnDrop(()));
        #[allow(clippy::let_unit_value)]
        let _: () = transmute!(#![allow(shrink)] PanicOnDrop(()));

        // Test that `transmute!` is legal in a const context.
        const ARRAY_OF_U8S: [u8; 8] = [0u8, 1, 2, 3, 4, 5, 6, 7];
        const ARRAY_OF_ARRAYS: [[u8; 2]; 4] = [[0, 1], [2, 3], [4, 5], [6, 7]];
        const X: [[u8; 2]; 4] = transmute!(ARRAY_OF_U8S);
        assert_eq!(X, ARRAY_OF_ARRAYS);
        const X_SHRINK: [[u8; 2]; 3] = transmute!(#![allow(shrink)] ARRAY_OF_U8S);
        assert_eq!(X_SHRINK, [[0u8, 1], [2, 3], [4, 5]]);

        // Test that `transmute!` works with `!Immutable` types.
        let x: usize = transmute!(UnsafeCell::new(1usize));
        assert_eq!(x, 1);
        let x: UnsafeCell<usize> = transmute!(1usize);
        assert_eq!(x.into_inner(), 1);
        let x: UnsafeCell<isize> = transmute!(UnsafeCell::new(1usize));
        assert_eq!(x.into_inner(), 1);
    }

    // A `Sized` type which doesn't implement `KnownLayout` (it is "not
    // `KnownLayout`", or `Nkl`).
    //
    // This permits us to test that `transmute_ref!` and `transmute_mut!` work
    // for types which are `Sized + !KnownLayout`. When we added support for
    // slice DSTs in #1924, this new support relied on `KnownLayout`, but we
    // need to make sure to remain backwards-compatible with code which uses
    // these macros with types which are `!KnownLayout`.
    #[derive(FromBytes, IntoBytes, Immutable, PartialEq, Eq, Debug)]
    #[repr(transparent)]
    struct Nkl<T>(T);

    #[test]
    fn test_transmute_ref() {
        // Test that memory is transmuted as expected.
        let array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let x: &[[u8; 2]; 4] = transmute_ref!(&array_of_u8s);
        assert_eq!(*x, array_of_arrays);
        let x: &[u8; 8] = transmute_ref!(&array_of_arrays);
        assert_eq!(*x, array_of_u8s);

        // Test that `transmute_ref!` is legal in a const context.
        const ARRAY_OF_U8S: [u8; 8] = [0u8, 1, 2, 3, 4, 5, 6, 7];
        const ARRAY_OF_ARRAYS: [[u8; 2]; 4] = [[0, 1], [2, 3], [4, 5], [6, 7]];
        #[allow(clippy::redundant_static_lifetimes)]
        const X: &'static [[u8; 2]; 4] = transmute_ref!(&ARRAY_OF_U8S);
        assert_eq!(*X, ARRAY_OF_ARRAYS);

        // Test sized -> unsized transmutation.
        let array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let slice_of_arrays = &array_of_arrays[..];
        let x: &[[u8; 2]] = transmute_ref!(&array_of_u8s);
        assert_eq!(x, slice_of_arrays);

        // Before 1.61.0, we can't define the `const fn transmute_ref` function
        // that we do on and after 1.61.0.
        #[cfg(no_zerocopy_generic_bounds_in_const_fn_1_61_0)]
        {
            // Test that `transmute_ref!` supports non-`KnownLayout` `Sized`
            // types.
            const ARRAY_OF_NKL_U8S: Nkl<[u8; 8]> = Nkl([0u8, 1, 2, 3, 4, 5, 6, 7]);
            const ARRAY_OF_NKL_ARRAYS: Nkl<[[u8; 2]; 4]> = Nkl([[0, 1], [2, 3], [4, 5], [6, 7]]);
            const X_NKL: &Nkl<[[u8; 2]; 4]> = transmute_ref!(&ARRAY_OF_NKL_U8S);
            assert_eq!(*X_NKL, ARRAY_OF_NKL_ARRAYS);
        }

        #[cfg(not(no_zerocopy_generic_bounds_in_const_fn_1_61_0))]
        {
            // Call through a generic function to make sure our autoref
            // specialization trick works even when types are generic.
            const fn transmute_ref<T, U>(t: &T) -> &U
            where
                T: IntoBytes + Immutable,
                U: FromBytes + Immutable,
            {
                transmute_ref!(t)
            }

            // Test that `transmute_ref!` supports non-`KnownLayout` `Sized`
            // types.
            const ARRAY_OF_NKL_U8S: Nkl<[u8; 8]> = Nkl([0u8, 1, 2, 3, 4, 5, 6, 7]);
            const ARRAY_OF_NKL_ARRAYS: Nkl<[[u8; 2]; 4]> = Nkl([[0, 1], [2, 3], [4, 5], [6, 7]]);
            const X_NKL: &Nkl<[[u8; 2]; 4]> = transmute_ref(&ARRAY_OF_NKL_U8S);
            assert_eq!(*X_NKL, ARRAY_OF_NKL_ARRAYS);
        }

        // Test that `transmute_ref!` works on slice DSTs in and that memory is
        // transmuted as expected.
        let slice_dst_of_u8s =
            SliceDst::<U16, [u8; 2]>::ref_from_bytes(&[0, 1, 2, 3, 4, 5][..]).unwrap();
        let slice_dst_of_u16s =
            SliceDst::<U16, U16>::ref_from_bytes(&[0, 1, 2, 3, 4, 5][..]).unwrap();
        let x: &SliceDst<U16, U16> = transmute_ref!(slice_dst_of_u8s);
        assert_eq!(x, slice_dst_of_u16s);

        let slice_dst_of_u8s =
            SliceDst::<U16, u8>::ref_from_bytes(&[0, 1, 2, 3, 4, 5][..]).unwrap();
        let x: &[u8] = transmute_ref!(slice_dst_of_u8s);
        assert_eq!(x, [0, 1, 2, 3, 4, 5]);

        let x: &[u8] = transmute_ref!(slice_dst_of_u16s);
        assert_eq!(x, [0, 1, 2, 3, 4, 5]);

        let x: &[U16] = transmute_ref!(slice_dst_of_u16s);
        let slice_of_u16s: &[U16] = <[U16]>::ref_from_bytes(&[0, 1, 2, 3, 4, 5][..]).unwrap();
        assert_eq!(x, slice_of_u16s);

        // Test that transmuting from a type with larger trailing slice offset
        // and larger trailing slice element works.
        let bytes = &[0, 1, 2, 3, 4, 5, 6, 7][..];
        let slice_dst_big = SliceDst::<U32, U16>::ref_from_bytes(bytes).unwrap();
        let slice_dst_small = SliceDst::<U16, u8>::ref_from_bytes(bytes).unwrap();
        let x: &SliceDst<U16, u8> = transmute_ref!(slice_dst_big);
        assert_eq!(x, slice_dst_small);

        // A packed outer DST can preserve the trailing field's stronger
        // rounding alignment. These two types consequently have different
        // normalized layout formulas even though both have size `2 + 4 * n`
        // for every trailing slice length. Ensure the semantic size-sequence
        // comparison accepts this representation-preserving transmutation.
        #[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
        #[repr(C, align(2))]
        struct TransmuteAlign2([u8; 4]);

        #[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
        #[repr(C, packed(2))]
        struct TransmutePacked4 {
            head: u16,
            tail: [u32],
        }

        #[derive(FromBytes, Immutable, IntoBytes, KnownLayout)]
        #[repr(C, packed(2))]
        struct TransmutePacked2 {
            head: u16,
            tail: [TransmuteAlign2],
        }

        #[repr(C, align(2))]
        struct TransmuteBytes([u8; 10]);

        let bytes = TransmuteBytes([0; 10]);
        let src = TransmutePacked4::ref_from_bytes(&bytes.0).unwrap();
        let dst: &TransmutePacked2 = transmute_ref!(src);
        assert_eq!(mem::size_of_val(src), 10);
        assert_eq!(mem::size_of_val(dst), 10);

        // Test that it's legal to transmute a reference while shrinking the
        // lifetime (note that `X` has the lifetime `'static`).
        let x: &[u8; 8] = transmute_ref!(X);
        assert_eq!(*x, ARRAY_OF_U8S);

        // Test that `transmute_ref!` supports decreasing alignment.
        let u = AU64(0);
        let array = [0, 0, 0, 0, 0, 0, 0, 0];
        let x: &[u8; 8] = transmute_ref!(&u);
        assert_eq!(*x, array);

        // Test that a mutable reference can be turned into an immutable one.
        let mut x = 0u8;
        #[allow(clippy::useless_transmute)]
        let y: &u8 = transmute_ref!(&mut x);
        assert_eq!(*y, 0);
    }

    #[test]
    fn test_try_transmute() {
        // Test that memory is transmuted with `try_transmute` as expected.
        let array_of_bools = [false, true, false, true, false, true, false, true];
        let array_of_arrays = [[0, 1], [0, 1], [0, 1], [0, 1]];
        let x: Result<[[u8; 2]; 4], _> = try_transmute!(array_of_bools);
        assert_eq!(x, Ok(array_of_arrays));
        let x: Result<[bool; 8], _> = try_transmute!(array_of_arrays);
        assert_eq!(x, Ok(array_of_bools));

        // Test that `try_transmute!` works with `!Immutable` types.
        let x: Result<usize, _> = try_transmute!(UnsafeCell::new(1usize));
        assert_eq!(x.unwrap(), 1);
        let x: Result<UnsafeCell<usize>, _> = try_transmute!(1usize);
        assert_eq!(x.unwrap().into_inner(), 1);
        let x: Result<UnsafeCell<isize>, _> = try_transmute!(UnsafeCell::new(1usize));
        assert_eq!(x.unwrap().into_inner(), 1);

        #[derive(FromBytes, IntoBytes, Debug, PartialEq)]
        #[repr(transparent)]
        struct PanicOnDrop<T>(T);

        impl<T> Drop for PanicOnDrop<T> {
            fn drop(&mut self) {
                panic!("PanicOnDrop dropped");
            }
        }

        // Since `try_transmute!` semantically moves its argument on failure,
        // the `PanicOnDrop` is not dropped, and thus this shouldn't panic.
        let x: Result<usize, _> = try_transmute!(PanicOnDrop(1usize));
        assert_eq!(x, Ok(1));

        // Since `try_transmute!` semantically returns ownership of its argument
        // on failure, the `PanicOnDrop` is returned rather than dropped, and
        // thus this shouldn't panic.
        let y: Result<bool, _> = try_transmute!(PanicOnDrop(2u8));
        // We have to use `map_err` instead of comparing against
        // `Err(PanicOnDrop(2u8))` because the latter would create and then drop
        // its `PanicOnDrop` temporary, which would cause a panic.
        assert_eq!(y.as_ref().map_err(|p| &p.src.0), Err::<&bool, _>(&2u8));
        mem::forget(y);

        #[derive(Debug, TryFromBytes)]
        #[repr(C, align(8))]
        struct PaddedBool(bool);

        // A failed transmutation must return every source byte, including
        // bytes which would be padding in the destination type.
        let src = [2u8, 1, 2, 3, 4, 5, 6, 7];
        let x: Result<PaddedBool, _> = try_transmute!(src);
        assert_eq!(x.unwrap_err().into_src(), src);

        let x: Result<PaddedBool, _> = try_transmute!([0u8; 8]);
        assert!(!x.unwrap().0);
    }

    #[test]
    fn test_try_transmute_ref() {
        // Test that memory is transmuted with `try_transmute_ref` as expected.
        let array_of_bools = &[false, true, false, true, false, true, false, true];
        let array_of_arrays = &[[0, 1], [0, 1], [0, 1], [0, 1]];
        let x: Result<&[[u8; 2]; 4], _> = try_transmute_ref!(array_of_bools);
        assert_eq!(x, Ok(array_of_arrays));
        let x: Result<&[bool; 8], _> = try_transmute_ref!(array_of_arrays);
        assert_eq!(x, Ok(array_of_bools));

        // Test that it's legal to transmute a reference while shrinking the
        // lifetime.
        {
            let x: Result<&[[u8; 2]; 4], _> = try_transmute_ref!(array_of_bools);
            assert_eq!(x, Ok(array_of_arrays));
        }

        // Test that `try_transmute_ref!` supports decreasing alignment.
        let u = AU64(0);
        let array = [0u8, 0, 0, 0, 0, 0, 0, 0];
        let x: Result<&[u8; 8], _> = try_transmute_ref!(&u);
        assert_eq!(x, Ok(&array));

        // Test that a mutable reference can be turned into an immutable one.
        let mut x = 0u8;
        #[allow(clippy::useless_transmute)]
        let y: Result<&u8, _> = try_transmute_ref!(&mut x);
        assert_eq!(y, Ok(&0));

        // Test that sized types work which don't implement `KnownLayout`.
        let array_of_nkl_u8s = Nkl([0u8, 1, 2, 3, 4, 5, 6, 7]);
        let array_of_nkl_arrays = Nkl([[0, 1], [2, 3], [4, 5], [6, 7]]);
        let x: Result<&Nkl<[[u8; 2]; 4]>, _> = try_transmute_ref!(&array_of_nkl_u8s);
        assert_eq!(x, Ok(&array_of_nkl_arrays));

        // Test sized -> unsized transmutation.
        let array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let slice_of_arrays = &array_of_arrays[..];
        let x: Result<&[[u8; 2]], _> = try_transmute_ref!(&array_of_u8s);
        assert_eq!(x, Ok(slice_of_arrays));

        // Test unsized -> unsized transmutation.
        let slice_dst_of_u8s =
            SliceDst::<U16, [u8; 2]>::ref_from_bytes(&[0, 1, 2, 3, 4, 5][..]).unwrap();
        let slice_dst_of_u16s =
            SliceDst::<U16, U16>::ref_from_bytes(&[0, 1, 2, 3, 4, 5][..]).unwrap();
        let x: Result<&SliceDst<U16, U16>, _> = try_transmute_ref!(slice_dst_of_u8s);
        assert_eq!(x, Ok(slice_dst_of_u16s));
    }

    #[test]
    fn test_try_transmute_mut() {
        // Test that memory is transmuted with `try_transmute_mut` as expected.
        let array_of_u8s = &mut [0u8, 1, 0, 1, 0, 1, 0, 1];
        let array_of_arrays = &mut [[0u8, 1], [0, 1], [0, 1], [0, 1]];
        let x: Result<&mut [[u8; 2]; 4], _> = try_transmute_mut!(array_of_u8s);
        assert_eq!(x, Ok(array_of_arrays));

        let array_of_bools = &mut [false, true, false, true, false, true, false, true];
        let array_of_arrays = &mut [[0u8, 1], [0, 1], [0, 1], [0, 1]];
        let x: Result<&mut [bool; 8], _> = try_transmute_mut!(array_of_arrays);
        assert_eq!(x, Ok(array_of_bools));

        // Test that it's legal to transmute a reference while shrinking the
        // lifetime.
        let array_of_bools = &mut [false, true, false, true, false, true, false, true];
        let array_of_arrays = &mut [[0u8, 1], [0, 1], [0, 1], [0, 1]];
        {
            let x: Result<&mut [bool; 8], _> = try_transmute_mut!(array_of_arrays);
            assert_eq!(x, Ok(array_of_bools));
        }

        // Test that `try_transmute_mut!` supports decreasing alignment.
        let u = &mut AU64(0);
        let array = &mut [0u8, 0, 0, 0, 0, 0, 0, 0];
        let x: Result<&mut [u8; 8], _> = try_transmute_mut!(u);
        assert_eq!(x, Ok(array));

        // Test that a mutable reference can be turned into an immutable one.
        let mut x = 0u8;
        #[allow(clippy::useless_transmute)]
        let y: Result<&mut u8, _> = try_transmute_mut!(&mut x);
        assert_eq!(y, Ok(&mut 0));

        // Test that sized types work which don't implement `KnownLayout`.
        let mut array_of_nkl_u8s = Nkl([0u8, 1, 2, 3, 4, 5, 6, 7]);
        let mut array_of_nkl_arrays = Nkl([[0, 1], [2, 3], [4, 5], [6, 7]]);
        let x: Result<&mut Nkl<[[u8; 2]; 4]>, _> = try_transmute_mut!(&mut array_of_nkl_u8s);
        assert_eq!(x, Ok(&mut array_of_nkl_arrays));

        // Test sized -> unsized transmutation.
        let mut array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let mut array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let slice_of_arrays = &mut array_of_arrays[..];
        let x: Result<&mut [[u8; 2]], _> = try_transmute_mut!(&mut array_of_u8s);
        assert_eq!(x, Ok(slice_of_arrays));

        // Test unsized -> unsized transmutation.
        let mut bytes = [0, 1, 2, 3, 4, 5, 6];
        let slice_dst_of_u8s = SliceDst::<u8, [u8; 2]>::mut_from_bytes(&mut bytes[..]).unwrap();
        let mut bytes = [0, 1, 2, 3, 4, 5, 6];
        let slice_dst_of_u16s = SliceDst::<u8, U16>::mut_from_bytes(&mut bytes[..]).unwrap();
        let x: Result<&mut SliceDst<u8, U16>, _> = try_transmute_mut!(slice_dst_of_u8s);
        assert_eq!(x, Ok(slice_dst_of_u16s));
    }

    #[test]
    fn test_transmute_mut() {
        // Test that memory is transmuted as expected.
        let mut array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let mut array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let x: &mut [[u8; 2]; 4] = transmute_mut!(&mut array_of_u8s);
        assert_eq!(*x, array_of_arrays);
        let x: &mut [u8; 8] = transmute_mut!(&mut array_of_arrays);
        assert_eq!(*x, array_of_u8s);

        {
            // Test that it's legal to transmute a reference while shrinking the
            // lifetime.
            let x: &mut [u8; 8] = transmute_mut!(&mut array_of_arrays);
            assert_eq!(*x, array_of_u8s);
        }

        // Test that `transmute_mut!` supports non-`KnownLayout` types.
        let mut array_of_u8s = Nkl([0u8, 1, 2, 3, 4, 5, 6, 7]);
        let mut array_of_arrays = Nkl([[0, 1], [2, 3], [4, 5], [6, 7]]);
        let x: &mut Nkl<[[u8; 2]; 4]> = transmute_mut!(&mut array_of_u8s);
        assert_eq!(*x, array_of_arrays);
        let x: &mut Nkl<[u8; 8]> = transmute_mut!(&mut array_of_arrays);
        assert_eq!(*x, array_of_u8s);

        // Test that `transmute_mut!` supports decreasing alignment.
        let mut u = AU64(0);
        let array = [0, 0, 0, 0, 0, 0, 0, 0];
        let x: &[u8; 8] = transmute_mut!(&mut u);
        assert_eq!(*x, array);

        // Test that a mutable reference can be turned into an immutable one.
        let mut x = 0u8;
        #[allow(clippy::useless_transmute)]
        let y: &u8 = transmute_mut!(&mut x);
        assert_eq!(*y, 0);

        // Test that `transmute_mut!` works on slice DSTs in and that memory is
        // transmuted as expected.
        let mut bytes = [0, 1, 2, 3, 4, 5, 6];
        let slice_dst_of_u8s = SliceDst::<u8, [u8; 2]>::mut_from_bytes(&mut bytes[..]).unwrap();
        let mut bytes = [0, 1, 2, 3, 4, 5, 6];
        let slice_dst_of_u16s = SliceDst::<u8, U16>::mut_from_bytes(&mut bytes[..]).unwrap();
        let x: &mut SliceDst<u8, U16> = transmute_mut!(slice_dst_of_u8s);
        assert_eq!(x, slice_dst_of_u16s);

        // Test that `transmute_mut!` works on slices that memory is transmuted
        // as expected.
        let array_of_u16s: &mut [u16] = &mut [0u16, 1, 2];
        let array_of_i16s: &mut [i16] = &mut [0i16, 1, 2];
        let x: &mut [i16] = transmute_mut!(array_of_u16s);
        assert_eq!(x, array_of_i16s);

        // Test that transmuting from a type with larger trailing slice offset
        // and larger trailing slice element works.
        let mut bytes = [0, 1, 2, 3, 4, 5, 6, 7];
        let slice_dst_big = SliceDst::<U32, U16>::mut_from_bytes(&mut bytes[..]).unwrap();
        let mut bytes = [0, 1, 2, 3, 4, 5, 6, 7];
        let slice_dst_small = SliceDst::<U16, u8>::mut_from_bytes(&mut bytes[..]).unwrap();
        let x: &mut SliceDst<U16, u8> = transmute_mut!(slice_dst_big);
        assert_eq!(x, slice_dst_small);

        // Test sized -> unsized transmutation.
        let mut array_of_u8s = [0u8, 1, 2, 3, 4, 5, 6, 7];
        let mut array_of_arrays = [[0, 1], [2, 3], [4, 5], [6, 7]];
        let slice_of_arrays = &mut array_of_arrays[..];
        let x: &mut [[u8; 2]] = transmute_mut!(&mut array_of_u8s);
        assert_eq!(x, slice_of_arrays);
    }

    #[test]
    fn test_macros_evaluate_args_once() {
        let mut ctr = 0;
        #[allow(clippy::useless_transmute)]
        let _: usize = transmute!({
            ctr += 1;
            0usize
        });
        assert_eq!(ctr, 1);

        let mut ctr = 0;
        let _: &usize = transmute_ref!({
            ctr += 1;
            &0usize
        });
        assert_eq!(ctr, 1);

        let mut ctr: usize = 0;
        let _: &mut usize = transmute_mut!({
            ctr += 1;
            &mut ctr
        });
        assert_eq!(ctr, 1);

        let mut ctr = 0;
        #[allow(clippy::useless_transmute)]
        let _: usize = try_transmute!({
            ctr += 1;
            0usize
        })
        .unwrap();
        assert_eq!(ctr, 1);
    }

    #[test]
    fn test_include_value() {
        const AS_U32: u32 = include_value!("../../../testdata/include_value/data");
        assert_eq!(AS_U32, u32::from_ne_bytes(*b"abcd"));
        const AS_I32: i32 = include_value!("../../../testdata/include_value/data");
        assert_eq!(AS_I32, i32::from_ne_bytes(*b"abcd"));
    }

    #[test]
    #[allow(non_camel_case_types, unreachable_pub, dead_code)]
    fn test_cryptocorrosion_derive_traits() {
        // Test the set of invocations added in
        // https://github.com/cryptocorrosion/cryptocorrosion/pull/85

        fn assert_impls<T: FromBytes + IntoBytes + Immutable>() {}

        cryptocorrosion_derive_traits! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub union vec128_storage {
                d: [u32; 4],
                q: [u64; 2],
            }
        }

        assert_impls::<vec128_storage>();

        cryptocorrosion_derive_traits! {
            #[repr(transparent)]
            #[derive(Copy, Clone, Debug, PartialEq)]
            pub struct u32x4_generic([u32; 4]);
        }

        assert_impls::<u32x4_generic>();

        cryptocorrosion_derive_traits! {
            #[repr(transparent)]
            #[derive(Copy, Clone, Debug, PartialEq)]
            pub struct u64x2_generic([u64; 2]);
        }

        assert_impls::<u64x2_generic>();

        cryptocorrosion_derive_traits! {
            #[repr(transparent)]
            #[derive(Copy, Clone, Debug, PartialEq)]
            pub struct u128x1_generic([u128; 1]);
        }

        assert_impls::<u128x1_generic>();

        cryptocorrosion_derive_traits! {
            #[repr(transparent)]
            #[derive(Copy, Clone, Default)]
            #[allow(non_camel_case_types)]
            pub struct x2<W, G>(pub [W; 2], PhantomData<G>);
        }

        enum NotZerocopy {}
        assert_impls::<x2<(), NotZerocopy>>();

        cryptocorrosion_derive_traits! {
            #[repr(transparent)]
            #[derive(Copy, Clone, Default)]
            #[allow(non_camel_case_types)]
            pub struct x4<W>(pub [W; 4]);
        }

        assert_impls::<x4<()>>();

        #[cfg(feature = "simd")]
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(target_arch = "x86")]
            use core::arch::x86::{__m128i, __m256i};
            #[cfg(target_arch = "x86_64")]
            use core::arch::x86_64::{__m128i, __m256i};

            cryptocorrosion_derive_traits! {
                #[repr(C)]
                #[derive(Copy, Clone)]
                pub struct X4(__m128i, __m128i, __m128i, __m128i);
            }

            assert_impls::<X4>();

            cryptocorrosion_derive_traits! {
                #[repr(C)]
                /// Generic wrapper for unparameterized storage of any of the
                /// possible impls. Converting into and out of this type should
                /// be essentially free, although it may be more aligned than a
                /// particular impl requires.
                #[allow(non_camel_case_types)]
                #[derive(Copy, Clone)]
                pub union vec128_storage {
                    u32x4: [u32; 4],
                    u64x2: [u64; 2],
                    u128x1: [u128; 1],
                    sse2: __m128i,
                }
            }

            assert_impls::<vec128_storage>();

            cryptocorrosion_derive_traits! {
                #[repr(transparent)]
                #[allow(non_camel_case_types)]
                #[derive(Copy, Clone)]
                pub struct vec<S3, S4, NI> {
                    x: __m128i,
                    s3: PhantomData<S3>,
                    s4: PhantomData<S4>,
                    ni: PhantomData<NI>,
                }
            }

            assert_impls::<vec<NotZerocopy, NotZerocopy, NotZerocopy>>();

            cryptocorrosion_derive_traits! {
                #[repr(transparent)]
                #[derive(Copy, Clone)]
                pub struct u32x4x2_avx2<NI> {
                    x: __m256i,
                    ni: PhantomData<NI>,
                }
            }

            assert_impls::<u32x4x2_avx2<NotZerocopy>>();
        }

        // Make sure that our derive works for `#[repr(C)]` structs even though
        // cryptocorrosion doesn't currently have any.
        cryptocorrosion_derive_traits! {
            #[repr(C)]
            #[derive(Copy, Clone, Debug, PartialEq)]
            pub struct ReprC(u8, u8, u16);
        }
    }
}
