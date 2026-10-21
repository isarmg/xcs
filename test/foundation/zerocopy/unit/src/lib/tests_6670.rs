#[cfg(test)]
#[allow(clippy::assertions_on_result_states, clippy::unreadable_literal)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;
    use crate::util::testutil::*;

    // An unsized type.
    //
    // This is used to test the custom derives of our traits. The `[u8]` type
    // gets a hand-rolled impl, so it doesn't exercise our custom derives.
    #[derive(Debug, Eq, PartialEq, FromBytes, IntoBytes, Unaligned, Immutable)]
    #[repr(transparent)]
    struct Unsized([u8]);

    impl Unsized {
        fn from_mut_slice(slc: &mut [u8]) -> &mut Unsized {
            // SAFETY: This *probably* sound - since the layouts of `[u8]` and
            // `Unsized` are the same, so are the layouts of `&mut [u8]` and
            // `&mut Unsized`. [1] Even if it turns out that this isn't actually
            // guaranteed by the language spec, we can just change this since
            // it's in test code.
            //
            // [1] https://github.com/rust-lang/unsafe-code-guidelines/issues/375
            unsafe { mem::transmute(slc) }
        }
    }

    // These types exercise a nested slice DST whose inner trailing-padding
    // operation cannot be flattened into the alignment of the packed outer
    // type. For `NestedPackedDstOuter<[u8]>`, the trailing slice begins at
    // offset 7, but the size for `elems` elements is
    // `2 + round_up(5 + elems, 4)`.
    //
    // The Reference rejects directly placing an alignment-4 field in a
    // `repr(packed(2))` type. Rust currently accepts this construction through
    // generic indirection. So long as Rust accepts it, this test deliberately
    // treats its layout as following the otherwise-applicable `repr(C)` and
    // `repr(packed)` rules.
    #[derive(FromBytes, Immutable, KnownLayout)]
    #[repr(C, align(4))]
    struct NestedPackedDstWord([u8; 4]);

    #[derive(FromBytes, Immutable, KnownLayout)]
    #[repr(C)]
    struct NestedPackedDstInner<T: ?Sized> {
        a: NestedPackedDstWord,
        b: u8,
        tail: T,
    }

    #[derive(FromBytes, Immutable, KnownLayout)]
    #[repr(C, packed(2))]
    struct NestedPackedDstOuter<T: ?Sized> {
        z: u8,
        inner: ManuallyDrop<NestedPackedDstInner<T>>,
    }

    #[repr(C, align(2))]
    struct Align2<T>(T);

    #[test]
    fn test_known_layout() {
        // Test that `$ty` and `ManuallyDrop<$ty>` have the expected layout.
        // Test that `PhantomData<$ty>` has the same layout as `()` regardless
        // of `$ty`.
        macro_rules! test {
            ($ty:ty, $expect:expr) => {
                let expect = $expect;
                assert_eq!(<$ty as KnownLayout>::LAYOUT, expect);
                assert_eq!(<ManuallyDrop<$ty> as KnownLayout>::LAYOUT, expect);
                assert_eq!(<PhantomData<$ty> as KnownLayout>::LAYOUT, <() as KnownLayout>::LAYOUT);
            };
        }

        let layout = |offset, align, trailing_slice_elem_size, statically_shallow_unpadded| {
            let align = NonZeroUsize::new(align).unwrap();
            let size_phase = offset % align;
            DstLayout {
                align,
                size_info: match trailing_slice_elem_size {
                    None => SizeInfo::Sized { size: offset },
                    Some(elem_size) => SizeInfo::SliceDst(TrailingSliceLayout {
                        offset,
                        elem_size,
                        size_base: offset - size_phase,
                        size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                            align, size_phase,
                        ),
                    }),
                },
                statically_shallow_unpadded,
            }
        };

        test!((), layout(0, 1, None, false));
        test!(u8, layout(1, 1, None, false));
        // Use `align_of` because `u64` alignment may be smaller than 8 on some
        // platforms.
        test!(u64, layout(8, mem::align_of::<u64>(), None, false));
        test!(AU64, layout(8, 8, None, false));

        test!(Option<&'static ()>, usize::LAYOUT);

        test!([()], layout(0, 1, Some(0), true));
        test!([u8], layout(0, 1, Some(1), true));
        test!(str, layout(0, 1, Some(1), true));
    }

    #[test]
    fn test_known_layout_pointer_to_metadata() {
        fn test<T>(data: *mut T, elems: usize) {
            let ptr = ptr::slice_from_raw_parts_mut(data, elems);
            assert_eq!(<[T] as KnownLayout>::pointer_to_metadata(ptr), elems);
        }

        for elems in [0, 1, usize::MAX] {
            test(ptr::null_mut::<u8>(), elems);
        }

        test(NonNull::<u8>::dangling().as_ptr(), 0);

        let mut bytes = [1, 2, 3];
        test(bytes.as_mut_ptr(), bytes.len());
        assert_eq!(bytes, [1, 2, 3]);

        test(NonNull::<()>::dangling().as_ptr(), usize::MAX);

        // Retaining and inspecting a raw pointer after freeing its allocation
        // is safe so long as the pointer is not dereferenced.
        let deallocated = {
            let mut byte = Box::new(0u8);
            let ptr: *mut u8 = &mut *byte;
            drop(byte);
            ptr
        };
        test(deallocated, 3);

        // Metadata extraction requires neither element storage nor alignment
        // for `T`; it does not access the pointer's referent.
        test(NonNull::<u8>::dangling().as_ptr().cast::<u64>(), usize::MAX);
    }



    #[test]
    fn test_nested_packed_dst_layout() {
        fn assert_native_size<const N: usize>(expected: usize) {
            let value = NestedPackedDstOuter {
                z: 0,
                inner: ManuallyDrop::new(NestedPackedDstInner {
                    a: NestedPackedDstWord([0; 4]),
                    b: 0,
                    tail: [0; N],
                }),
            };
            let value: &NestedPackedDstOuter<[u8]> = &value;
            assert_eq!(mem::size_of_val(value), expected);
            assert_eq!(NestedPackedDstOuter::<[u8]>::size_for_metadata(N), Some(expected));
        }

        assert_native_size::<0>(10);
        assert_native_size::<1>(10);
        assert_native_size::<2>(10);
        assert_native_size::<3>(10);
        assert_native_size::<4>(14);
        assert_native_size::<7>(14);
        assert_native_size::<8>(18);

        let layout = NestedPackedDstOuter::<[u8]>::LAYOUT;
        assert_eq!(layout.align.get(), 2);
        assert!(layout.requires_dynamic_padding());
        assert_eq!(
            layout.size_info,
            SizeInfo::SliceDst(TrailingSliceLayout {
                offset: 7,
                elem_size: 1,
                size_base: 6,
                size_rounding_align_and_phase: RoundingAlignAndPhase::new(
                    NonZeroUsize::new(4).unwrap(),
                    1
                ),
            })
        );

        // Eight bytes used to be incorrectly accepted for a zero-element
        // value. The actual Rust layout requires ten bytes.
        let short = Align2([0u8; 8]);
        assert!(NestedPackedDstOuter::<[u8]>::ref_from_bytes_with_elems(&short.0, 0).is_err());
        assert!(pointer::Ptr::from_ref(&short.0[..])
            .try_cast_into::<NestedPackedDstOuter<[u8]>, pointer::BecauseImmutable>(
                CastType::Prefix,
                Some(0),
            )
            .is_err());

        let full = Align2([0, 0, 0, 0, 0, 0, 0, 0, b'S', b'!']);
        let value = NestedPackedDstOuter::<[u8]>::ref_from_bytes_with_elems(&full.0, 0).unwrap();
        assert_eq!(mem::size_of_val(value), 10);

        // When metadata is inferred, lengths 0 through 3 all produce a
        // ten-byte object. Parsing chooses the largest trailing-slice length
        // which fits exactly.
        let inferred = NestedPackedDstOuter::<[u8]>::ref_from_bytes(&full.0).unwrap();
        assert_eq!(pointer::Ptr::from_ref(inferred).len(), 3);

        // Trailing padding includes the inner type's padding, even when the
        // outer type's alignment is reduced by packing.
        let trailing_layout = trailing_slice_layout::<NestedPackedDstOuter<[u8]>>();
        for (elems, expected) in [(0, 3), (1, 2), (2, 1), (3, 0), (4, 3)] {
            assert_eq!(trailing_layout.padding_for_elems(elems), expected);
        }
    }

    // Before Rust 1.76, rust-lang/rust#118537 caused `addr_of_mut!` to
    // miscompute the offset of an unsized field in a packed struct. The fix is
    // rust-lang/rust#118540. Keep the layout and parsing regression above on
    // all supported compilers, and gate only the projection regression which
    // necessarily exercises that independent compiler bug.
    #[rustversion::since(1.76.0)]
    #[test]
    fn test_nested_packed_dst_projection() {
        let full = Align2([0, 0, 0, 0, 0, 0, 0, 0, b'S', b'!']);

        // The projected inner value occupies the final eight bytes of the full
        // ten-byte input, including the two sentinel bytes.
        let (value, rest) = pointer::Ptr::from_ref(&full.0[..])
            .try_cast_into::<NestedPackedDstOuter<[u8]>, pointer::BecauseImmutable>(
                CastType::Prefix,
                Some(0),
            )
            .unwrap();
        assert!(rest.as_ref().is_empty());
        let inner = value.project::<project_clients::TryFromBytesDerive, _, { STRUCT_VARIANT_ID }, { ident_id!(inner) }>().unwrap();
        assert_eq!(
            inner.as_bytes::<pointer::BecauseImmutable>().as_ref(),
            &[0, 0, 0, 0, 0, 0, b'S', b'!']
        );
    }



    #[test]
    fn test_object_safety() {
        fn _takes_immutable(_: &dyn Immutable) {}
        fn _takes_unaligned(_: &dyn Unaligned) {}
    }

    #[test]
    fn test_from_zeros_only() {
        // Test types that implement `FromZeros` but not `FromBytes`.

        assert!(!bool::new_zeroed());
        assert_eq!(char::new_zeroed(), '\0');

        #[cfg(feature = "alloc")]
        {
            assert_eq!(bool::new_box_zeroed(), Ok(Box::new(false)));
            assert_eq!(char::new_box_zeroed(), Ok(Box::new('\0')));

            assert_eq!(
                <[bool]>::new_box_zeroed_with_elems(3).unwrap().as_ref(),
                [false, false, false]
            );
            assert_eq!(
                <[char]>::new_box_zeroed_with_elems(3).unwrap().as_ref(),
                ['\0', '\0', '\0']
            );

            assert_eq!(bool::new_vec_zeroed(3).unwrap().as_ref(), [false, false, false]);
            assert_eq!(char::new_vec_zeroed(3).unwrap().as_ref(), ['\0', '\0', '\0']);
        }

        let mut string = "hello".to_string();
        let s: &mut str = string.as_mut();
        assert_eq!(s, "hello");
        s.zero();
        assert_eq!(s, "\0\0\0\0\0");
    }

    #[test]
    fn test_zst_count_preserved() {
        // Test that, when an explicit count is provided to for a type with a
        // ZST trailing slice element, that count is preserved. This is
        // important since, for such types, all element counts result in objects
        // of the same size, and so the correct behavior is ambiguous. However,
        // preserving the count as requested by the user is the behavior that we
        // document publicly.

        // FromZeros methods
        #[cfg(feature = "alloc")]
        assert_eq!(<[()]>::new_box_zeroed_with_elems(3).unwrap().len(), 3);
        #[cfg(feature = "alloc")]
        assert_eq!(<()>::new_vec_zeroed(3).unwrap().len(), 3);

        // FromBytes methods
        assert_eq!(<[()]>::ref_from_bytes_with_elems(&[][..], 3).unwrap().len(), 3);
        assert_eq!(<[()]>::ref_from_prefix_with_elems(&[][..], 3).unwrap().0.len(), 3);
        assert_eq!(<[()]>::ref_from_suffix_with_elems(&[][..], 3).unwrap().1.len(), 3);
        assert_eq!(<[()]>::mut_from_bytes_with_elems(&mut [][..], 3).unwrap().len(), 3);
        assert_eq!(<[()]>::mut_from_prefix_with_elems(&mut [][..], 3).unwrap().0.len(), 3);
        assert_eq!(<[()]>::mut_from_suffix_with_elems(&mut [][..], 3).unwrap().1.len(), 3);
    }

    #[test]
    fn test_read_write() {
        const VAL: u64 = 0x12345678;
        #[cfg(target_endian = "big")]
        const VAL_BYTES: [u8; 8] = VAL.to_be_bytes();
        #[cfg(target_endian = "little")]
        const VAL_BYTES: [u8; 8] = VAL.to_le_bytes();
        const ZEROS: [u8; 8] = [0u8; 8];

        // Test `FromBytes::{read_from, read_from_prefix, read_from_suffix}`.

        assert_eq!(u64::read_from_bytes(&VAL_BYTES[..]), Ok(VAL));
        // The first 8 bytes are from `VAL_BYTES` and the second 8 bytes are all
        // zeros.
        let bytes_with_prefix: [u8; 16] = transmute!([VAL_BYTES, [0; 8]]);
        assert_eq!(u64::read_from_prefix(&bytes_with_prefix[..]), Ok((VAL, &ZEROS[..])));
        assert_eq!(u64::read_from_suffix(&bytes_with_prefix[..]), Ok((&VAL_BYTES[..], 0)));
        // The first 8 bytes are all zeros and the second 8 bytes are from
        // `VAL_BYTES`
        let bytes_with_suffix: [u8; 16] = transmute!([[0; 8], VAL_BYTES]);
        assert_eq!(u64::read_from_prefix(&bytes_with_suffix[..]), Ok((0, &VAL_BYTES[..])));
        assert_eq!(u64::read_from_suffix(&bytes_with_suffix[..]), Ok((&ZEROS[..], VAL)));

        // Test `IntoBytes::{write_to, write_to_prefix, write_to_suffix}`.

        let mut bytes = [0u8; 8];
        assert_eq!(VAL.write_to(&mut bytes[..]), Ok(()));
        assert_eq!(bytes, VAL_BYTES);
        let mut bytes = [0u8; 16];
        assert_eq!(VAL.write_to_prefix(&mut bytes[..]), Ok(()));
        let want: [u8; 16] = transmute!([VAL_BYTES, [0; 8]]);
        assert_eq!(bytes, want);
        let mut bytes = [0u8; 16];
        assert_eq!(VAL.write_to_suffix(&mut bytes[..]), Ok(()));
        let want: [u8; 16] = transmute!([[0; 8], VAL_BYTES]);
        assert_eq!(bytes, want);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_read_io_with_padding_soundness() {
        // This test is designed to exhibit potential UB in
        // `FromBytes::read_from_io`. (see #2319, #2320).

        // On most platforms (where `align_of::<u16>() == 2`), `WithPadding`
        // will have inter-field padding between `x` and `y`.
        #[derive(FromBytes)]
        #[repr(C)]
        struct WithPadding {
            x: u8,
            y: u16,
        }
        struct ReadsInRead;
        impl std::io::Read for ReadsInRead {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                // This body branches on every byte of `buf`, ensuring that it
                // exhibits UB if any byte of `buf` is uninitialized.
                if buf.iter().all(|&x| x == 0) {
                    Ok(buf.len())
                } else {
                    buf.iter_mut().for_each(|x| *x = 0);
                    Ok(buf.len())
                }
            }
        }
        assert!(matches!(WithPadding::read_from_io(ReadsInRead), Ok(WithPadding { x: 0, y: 0 })));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_read_write_io() {
        let mut long_buffer = [0, 0, 0, 0];
        assert!(matches!(u16::MAX.write_to_io(&mut long_buffer[..]), Ok(())));
        assert_eq!(long_buffer, [255, 255, 0, 0]);
        assert!(matches!(u16::read_from_io(&long_buffer[..]), Ok(u16::MAX)));

        let mut short_buffer = [0, 0];
        assert!(u32::MAX.write_to_io(&mut short_buffer[..]).is_err());
        assert_eq!(short_buffer, [255, 255]);
        assert!(u32::read_from_io(&short_buffer[..]).is_err());
    }

    #[test]
    fn test_try_from_bytes_try_read_from() {
        assert_eq!(<bool as TryFromBytes>::try_read_from_bytes(&[0]), Ok(false));
        assert_eq!(<bool as TryFromBytes>::try_read_from_bytes(&[1]), Ok(true));

        assert_eq!(<bool as TryFromBytes>::try_read_from_prefix(&[0, 2]), Ok((false, &[2][..])));
        assert_eq!(<bool as TryFromBytes>::try_read_from_prefix(&[1, 2]), Ok((true, &[2][..])));

        assert_eq!(<bool as TryFromBytes>::try_read_from_suffix(&[2, 0]), Ok((&[2][..], false)));
        assert_eq!(<bool as TryFromBytes>::try_read_from_suffix(&[2, 1]), Ok((&[2][..], true)));

        // If we don't pass enough bytes, it fails.
        assert!(matches!(
            <u8 as TryFromBytes>::try_read_from_bytes(&[]),
            Err(TryReadError::Size(_))
        ));
        assert!(matches!(
            <u8 as TryFromBytes>::try_read_from_prefix(&[]),
            Err(TryReadError::Size(_))
        ));
        assert!(matches!(
            <u8 as TryFromBytes>::try_read_from_suffix(&[]),
            Err(TryReadError::Size(_))
        ));

        // If we pass too many bytes, it fails.
        assert!(matches!(
            <u8 as TryFromBytes>::try_read_from_bytes(&[0, 0]),
            Err(TryReadError::Size(_))
        ));

        // If we pass an invalid value, it fails.
        assert!(matches!(
            <bool as TryFromBytes>::try_read_from_bytes(&[2]),
            Err(TryReadError::Validity(_))
        ));
        assert!(matches!(
            <bool as TryFromBytes>::try_read_from_prefix(&[2, 0]),
            Err(TryReadError::Validity(_))
        ));
        assert!(matches!(
            <bool as TryFromBytes>::try_read_from_suffix(&[0, 2]),
            Err(TryReadError::Validity(_))
        ));

        // Reading from a misaligned buffer should still succeed. Since `AU64`'s
        // alignment is 8, and since we read from two adjacent addresses one
        // byte apart, it is guaranteed that at least one of them (though
        // possibly both) will be misaligned.
        let bytes: [u8; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(<AU64 as TryFromBytes>::try_read_from_bytes(&bytes[..8]), Ok(AU64(0)));
        assert_eq!(<AU64 as TryFromBytes>::try_read_from_bytes(&bytes[1..9]), Ok(AU64(0)));

        assert_eq!(
            <AU64 as TryFromBytes>::try_read_from_prefix(&bytes[..8]),
            Ok((AU64(0), &[][..]))
        );
        assert_eq!(
            <AU64 as TryFromBytes>::try_read_from_prefix(&bytes[1..9]),
            Ok((AU64(0), &[][..]))
        );

        assert_eq!(
            <AU64 as TryFromBytes>::try_read_from_suffix(&bytes[..8]),
            Ok((&[][..], AU64(0)))
        );
        assert_eq!(
            <AU64 as TryFromBytes>::try_read_from_suffix(&bytes[1..9]),
            Ok((&[][..], AU64(0)))
        );
    }

    #[test]
    fn test_try_read_from_typed_source() {
        #[derive(IntoBytes, Immutable)]
        #[repr(transparent)]
        struct Source<T: ?Sized>(T);

        fn check<S: IntoBytes + Immutable + ?Sized>(source: &S, valid: bool) {
            match try_read_from::<_, [bool; 4]>(source) {
                Ok(value) => {
                    assert!(valid);
                    assert_eq!(value, [false, true, false, true]);
                }
                Err(error) => {
                    assert!(!valid);
                    assert!(matches!(error, TryReadError::Validity(_)));
                    assert!(ptr::eq(error.into_src(), source));
                }
            }

            let error = try_read_from::<_, [bool; 5]>(source).unwrap_err();
            assert!(matches!(error, TryReadError::Size(_)));
            assert!(ptr::eq(error.into_src(), source));
        }

        for (value, valid) in [(u16::from_ne_bytes([0, 1]), true), (0x0202, false)] {
            let source = Source([value; 2]);
            check(&source, valid);
            let source: &Source<[u16]> = &source;
            check(source, valid);
        }

        let source = Source([(); 2]);
        assert!(try_read_from::<_, ()>(&source).is_ok());
        let source: &Source<[()]> = &source;
        assert!(try_read_from::<_, ()>(source).is_ok());
    }

    #[test]
    fn test_try_read_initialized_padding() {
        #[derive(KnownLayout, Immutable, Debug, PartialEq, Eq)]
        #[repr(C, align(8))]
        struct Padded(u8);

        // SAFETY: `Padded` has only a `u8` field and padding, so all
        // initialized byte sequences are valid. Per
        // https://doc.rust-lang.org/1.93.1/reference/behavior-considered-undefined.html#invalid-values:
        //
        //   An integer (`i*`/`u*`), floating point value (`f*`), or raw pointer
        //   must be initialized, i.e., must not be obtained from uninitialized
        //   memory.
        //
        //   A `struct`, tuple, and array requires all fields/elements to be
        //   valid at their respective type.
        const _: () = unsafe {
            unsafe_impl!(=> TryFromBytes for Padded; |candidate| {
                // Observe every byte while validation is running, including
                // padding that a typed move of `MaybeUninit<Padded>` could
                // discard. The returned `Padded` need not retain its padding.
                assert_eq!(candidate.as_bytes::<BecauseImmutable>().as_ref(), &[0xA5; 8]);
                true
            })
        };

        let source = [0xA5; 8];
        assert_eq!(Padded::try_read_from_bytes(&source), Ok(Padded(0xA5)));

        let prefix_source = [0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0];
        let (value, suffix) = Padded::try_read_from_prefix(&prefix_source).unwrap();
        assert_eq!(value, Padded(0xA5));
        assert!(ptr::eq(suffix, &prefix_source[8..]));

        let suffix_source = [0, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5, 0xA5];
        let (prefix, value) = Padded::try_read_from_suffix(&suffix_source).unwrap();
        assert_eq!(value, Padded(0xA5));
        assert!(ptr::eq(prefix, &suffix_source[..1]));
    }

    #[test]
    fn test_try_read_error_sources_and_zero_sized() {
        let invalid = [2, 3];
        for (error, source) in [
            (bool::try_read_from_bytes(&invalid[..1]).unwrap_err(), &invalid[..1]),
            (bool::try_read_from_prefix(&invalid).unwrap_err(), &invalid[..]),
            (bool::try_read_from_suffix(&invalid).unwrap_err(), &invalid[..]),
        ] {
            assert!(matches!(error, TryReadError::Validity(_)));
            assert!(ptr::eq(error.into_src(), source));
        }
        for source in [&[][..], &invalid[..]] {
            let error = bool::try_read_from_bytes(source).unwrap_err();
            assert!(matches!(error, TryReadError::Size(_)));
            assert!(ptr::eq(error.into_src(), source));
        }
        for error in [
            bool::try_read_from_prefix(&invalid[..0]).unwrap_err(),
            bool::try_read_from_suffix(&invalid[..0]).unwrap_err(),
        ] {
            assert!(matches!(error, TryReadError::Size(_)));
            assert!(ptr::eq(error.into_src(), &invalid[..0]));
        }

        assert_eq!(<()>::try_read_from_bytes(&[]), Ok(()));
        assert_eq!(<()>::try_read_from_prefix(&invalid), Ok(((), &invalid[..])));
        assert_eq!(<()>::try_read_from_suffix(&invalid), Ok((&invalid[..], ())));
    }

    #[test]
    fn test_ref_from_mut_from_bytes() {
        // Test `FromBytes::{ref_from_bytes, mut_from_bytes}{,_prefix,Suffix}`
        // success cases. Exhaustive coverage for these methods is covered by
        // the `Ref` tests above, which these helper methods defer to.

        let mut buf =
            Align::<[u8; 16], AU64>::new([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

        assert_eq!(
            AU64::ref_from_bytes(&buf.t[8..]).unwrap().0.to_ne_bytes(),
            [8, 9, 10, 11, 12, 13, 14, 15]
        );
        let suffix = AU64::mut_from_bytes(&mut buf.t[8..]).unwrap();
        suffix.0 = 0x0101010101010101;
        // The `[u8:9]` is a non-half size of the full buffer, which would catch
        // `from_prefix` having the same implementation as `from_suffix` (issues #506, #511).
        assert_eq!(
            <[u8; 9]>::ref_from_suffix(&buf.t[..]).unwrap(),
            (&[0, 1, 2, 3, 4, 5, 6][..], &[7u8, 1, 1, 1, 1, 1, 1, 1, 1])
        );
        let (prefix, suffix) = AU64::mut_from_suffix(&mut buf.t[1..]).unwrap();
        assert_eq!(prefix, &mut [1u8, 2, 3, 4, 5, 6, 7][..]);
        suffix.0 = 0x0202020202020202;
        let (prefix, suffix) = <[u8; 10]>::mut_from_suffix(&mut buf.t[..]).unwrap();
        assert_eq!(prefix, &mut [0u8, 1, 2, 3, 4, 5][..]);
        suffix[0] = 42;
        assert_eq!(
            <[u8; 9]>::ref_from_prefix(&buf.t[..]).unwrap(),
            (&[0u8, 1, 2, 3, 4, 5, 42, 7, 2], &[2u8, 2, 2, 2, 2, 2, 2][..])
        );
        <[u8; 2]>::mut_from_prefix(&mut buf.t[..]).unwrap().0[1] = 30;
        assert_eq!(buf.t, [0, 30, 2, 3, 4, 5, 42, 7, 2, 2, 2, 2, 2, 2, 2, 2]);
    }

    #[test]
    fn test_ref_from_mut_from_bytes_error() {
        // Test `FromBytes::{ref_from_bytes, mut_from_bytes}{,_prefix,Suffix}`
        // error cases.

        // Fail because the buffer is too large.
        let mut buf = Align::<[u8; 16], AU64>::default();
        // `buf.t` should be aligned to 8, so only the length check should fail.
        assert!(AU64::ref_from_bytes(&buf.t[..]).is_err());
        assert!(AU64::mut_from_bytes(&mut buf.t[..]).is_err());
        assert!(<[u8; 8]>::ref_from_bytes(&buf.t[..]).is_err());
        assert!(<[u8; 8]>::mut_from_bytes(&mut buf.t[..]).is_err());

        // Fail because the buffer is too small.
        let mut buf = Align::<[u8; 4], AU64>::default();
        assert!(AU64::ref_from_bytes(&buf.t[..]).is_err());
        assert!(AU64::mut_from_bytes(&mut buf.t[..]).is_err());
        assert!(<[u8; 8]>::ref_from_bytes(&buf.t[..]).is_err());
        assert!(<[u8; 8]>::mut_from_bytes(&mut buf.t[..]).is_err());
        assert!(AU64::ref_from_prefix(&buf.t[..]).is_err());
        assert!(AU64::mut_from_prefix(&mut buf.t[..]).is_err());
        assert!(AU64::ref_from_suffix(&buf.t[..]).is_err());
        assert!(AU64::mut_from_suffix(&mut buf.t[..]).is_err());
        assert!(<[u8; 8]>::ref_from_prefix(&buf.t[..]).is_err());
        assert!(<[u8; 8]>::mut_from_prefix(&mut buf.t[..]).is_err());
        assert!(<[u8; 8]>::ref_from_suffix(&buf.t[..]).is_err());
        assert!(<[u8; 8]>::mut_from_suffix(&mut buf.t[..]).is_err());

        // Fail because the alignment is insufficient.
        let mut buf = Align::<[u8; 13], AU64>::default();
        assert!(AU64::ref_from_bytes(&buf.t[1..]).is_err());
        assert!(AU64::mut_from_bytes(&mut buf.t[1..]).is_err());
        assert!(AU64::ref_from_bytes(&buf.t[1..]).is_err());
        assert!(AU64::mut_from_bytes(&mut buf.t[1..]).is_err());
        assert!(AU64::ref_from_prefix(&buf.t[1..]).is_err());
        assert!(AU64::mut_from_prefix(&mut buf.t[1..]).is_err());
        assert!(AU64::ref_from_suffix(&buf.t[..]).is_err());
        assert!(AU64::mut_from_suffix(&mut buf.t[..]).is_err());
    }

    #[test]
    fn test_to_methods() {
        /// Run a series of tests by calling `IntoBytes` methods on `t`.
        ///
        /// `bytes` is the expected byte sequence returned from `t.as_bytes()`
        /// before `t` has been modified. `post_mutation` is the expected
        /// sequence returned from `t.as_bytes()` after `t.as_mut_bytes()[0]`
        /// has had its bits flipped (by applying `^= 0xFF`).
        ///
        /// `N` is the size of `t` in bytes.
        fn test<T: FromBytes + IntoBytes + Immutable + Debug + Eq + ?Sized, const N: usize>(
            t: &mut T,
            bytes: &[u8],
            post_mutation: &T,
        ) {
            // Test that we can access the underlying bytes, and that we get the
            // right bytes and the right number of bytes.
            assert_eq!(t.as_bytes(), bytes);

            // Test that changes to the underlying byte slices are reflected in
            // the original object.
            t.as_mut_bytes()[0] ^= 0xFF;
            assert_eq!(t, post_mutation);
            t.as_mut_bytes()[0] ^= 0xFF;

            // `write_to` rejects slices that are too small or too large.
            assert!(t.write_to(&mut vec![0; N - 1][..]).is_err());
            assert!(t.write_to(&mut vec![0; N + 1][..]).is_err());

            // `write_to` works as expected.
            let mut bytes = [0; N];
            assert_eq!(t.write_to(&mut bytes[..]), Ok(()));
            assert_eq!(bytes, t.as_bytes());

            // `write_to_prefix` rejects slices that are too small.
            assert!(t.write_to_prefix(&mut vec![0; N - 1][..]).is_err());

            // `write_to_prefix` works with exact-sized slices.
            let mut bytes = [0; N];
            assert_eq!(t.write_to_prefix(&mut bytes[..]), Ok(()));
            assert_eq!(bytes, t.as_bytes());

            // `write_to_prefix` works with too-large slices, and any bytes past
            // the prefix aren't modified.
            let mut too_many_bytes = vec![0; N + 1];
            too_many_bytes[N] = 123;
            assert_eq!(t.write_to_prefix(&mut too_many_bytes[..]), Ok(()));
            assert_eq!(&too_many_bytes[..N], t.as_bytes());
            assert_eq!(too_many_bytes[N], 123);

            // `write_to_suffix` rejects slices that are too small.
            assert!(t.write_to_suffix(&mut vec![0; N - 1][..]).is_err());

            // `write_to_suffix` works with exact-sized slices.
            let mut bytes = [0; N];
            assert_eq!(t.write_to_suffix(&mut bytes[..]), Ok(()));
            assert_eq!(bytes, t.as_bytes());

            // `write_to_suffix` works with too-large slices, and any bytes
            // before the suffix aren't modified.
            let mut too_many_bytes = vec![0; N + 1];
            too_many_bytes[0] = 123;
            assert_eq!(t.write_to_suffix(&mut too_many_bytes[..]), Ok(()));
            assert_eq!(&too_many_bytes[1..], t.as_bytes());
            assert_eq!(too_many_bytes[0], 123);
        }

        #[derive(Debug, Eq, PartialEq, FromBytes, IntoBytes, Immutable)]
        #[repr(C)]
        struct Foo {
            a: u32,
            b: Wrapping<u32>,
            c: Option<NonZeroU32>,
        }

        let expected_bytes: Vec<u8> = if cfg!(target_endian = "little") {
            vec![1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]
        } else {
            vec![0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0]
        };
        let post_mutation_expected_a =
            if cfg!(target_endian = "little") { 0x00_00_00_FE } else { 0xFF_00_00_01 };
        test::<_, 12>(
            &mut Foo { a: 1, b: Wrapping(2), c: None },
            expected_bytes.as_bytes(),
            &Foo { a: post_mutation_expected_a, b: Wrapping(2), c: None },
        );
        test::<_, 3>(
            Unsized::from_mut_slice(&mut [1, 2, 3]),
            &[1, 2, 3],
            Unsized::from_mut_slice(&mut [0xFE, 2, 3]),
        );
    }

    #[test]
    fn test_array() {
        #[derive(FromBytes, IntoBytes, Immutable)]
        #[repr(C)]
        struct Foo {
            a: [u16; 33],
        }

        let foo = Foo { a: [0xFFFF; 33] };
        let expected = [0xFFu8; 66];
        assert_eq!(foo.as_bytes(), &expected[..]);
    }

    #[test]
    fn test_new_zeroed() {
        assert!(!bool::new_zeroed());
        assert_eq!(u64::new_zeroed(), 0);
        // This test exists in order to exercise unsafe code, especially when
        // running under Miri.
        #[allow(clippy::unit_cmp)]
        {
            assert_eq!(<()>::new_zeroed(), ());
        }
    }

    #[test]
    fn test_transparent_packed_generic_struct() {
        #[derive(IntoBytes, FromBytes, Unaligned)]
        #[repr(transparent)]
        #[allow(dead_code)] // We never construct this type
        struct Foo<T> {
            _t: T,
            _phantom: PhantomData<()>,
        }

        assert_impl_all!(Foo<u32>: FromZeros, FromBytes, IntoBytes);
        assert_impl_all!(Foo<u8>: Unaligned);

        #[derive(IntoBytes, FromBytes, Unaligned)]
        #[repr(C, packed)]
        #[allow(dead_code)] // We never construct this type
        struct Bar<T, U> {
            _t: T,
            _u: U,
        }

        assert_impl_all!(Bar<u8, AU64>: FromZeros, FromBytes, IntoBytes, Unaligned);
    }

    #[cfg(feature = "alloc")]
    mod alloc {
        use super::*;

        #[cfg(not(no_zerocopy_panic_in_const_and_vec_try_reserve_1_57_0))]
        #[test]
        fn test_extend_vec_zeroed() {
            // Test extending when there is an existing allocation.
            let mut v = vec![100u16, 200, 300];
            FromZeros::extend_vec_zeroed(&mut v, 3).unwrap();
            assert_eq!(v.len(), 6);
            assert_eq!(&*v, &[100, 200, 300, 0, 0, 0]);
            drop(v);

            // Test extending when there is no existing allocation.
            let mut v: Vec<u64> = Vec::new();
            FromZeros::extend_vec_zeroed(&mut v, 3).unwrap();
            assert_eq!(v.len(), 3);
            assert_eq!(&*v, &[0, 0, 0]);
            drop(v);
        }

        #[cfg(not(no_zerocopy_panic_in_const_and_vec_try_reserve_1_57_0))]
        #[test]
        fn test_extend_vec_zeroed_zst() {
            // Test extending when there is an existing (fake) allocation.
            let mut v = vec![(), (), ()];
            <()>::extend_vec_zeroed(&mut v, 3).unwrap();
            assert_eq!(v.len(), 6);
            assert_eq!(&*v, &[(), (), (), (), (), ()]);
            drop(v);

            // Test extending when there is no existing (fake) allocation.
            let mut v: Vec<()> = Vec::new();
            <()>::extend_vec_zeroed(&mut v, 3).unwrap();
            assert_eq!(&*v, &[(), (), ()]);
            drop(v);
        }

        #[cfg(not(no_zerocopy_panic_in_const_and_vec_try_reserve_1_57_0))]
        #[test]
        fn test_insert_vec_zeroed() {
            // Insert at start (no existing allocation).
            let mut v: Vec<u64> = Vec::new();
            u64::insert_vec_zeroed(&mut v, 0, 2).unwrap();
            assert_eq!(v.len(), 2);
            assert_eq!(&*v, &[0, 0]);
            drop(v);

            // Insert at start.
            let mut v = vec![100u64, 200, 300];
            u64::insert_vec_zeroed(&mut v, 0, 2).unwrap();
            assert_eq!(v.len(), 5);
            assert_eq!(&*v, &[0, 0, 100, 200, 300]);
            drop(v);

            // Insert at middle.
            let mut v = vec![100u64, 200, 300];
            u64::insert_vec_zeroed(&mut v, 1, 1).unwrap();
            assert_eq!(v.len(), 4);
            assert_eq!(&*v, &[100, 0, 200, 300]);
            drop(v);

            // Insert at end.
            let mut v = vec![100u64, 200, 300];
            u64::insert_vec_zeroed(&mut v, 3, 1).unwrap();
            assert_eq!(v.len(), 4);
            assert_eq!(&*v, &[100, 200, 300, 0]);
            drop(v);
        }

        #[cfg(not(no_zerocopy_panic_in_const_and_vec_try_reserve_1_57_0))]
        #[test]
        fn test_insert_vec_zeroed_zst() {
            // Insert at start (no existing fake allocation).
            let mut v: Vec<()> = Vec::new();
            <()>::insert_vec_zeroed(&mut v, 0, 2).unwrap();
            assert_eq!(v.len(), 2);
            assert_eq!(&*v, &[(), ()]);
            drop(v);

            // Insert at start.
            let mut v = vec![(), (), ()];
            <()>::insert_vec_zeroed(&mut v, 0, 2).unwrap();
            assert_eq!(v.len(), 5);
            assert_eq!(&*v, &[(), (), (), (), ()]);
            drop(v);

            // Insert at middle.
            let mut v = vec![(), (), ()];
            <()>::insert_vec_zeroed(&mut v, 1, 1).unwrap();
            assert_eq!(v.len(), 4);
            assert_eq!(&*v, &[(), (), (), ()]);
            drop(v);

            // Insert at end.
            let mut v = vec![(), (), ()];
            <()>::insert_vec_zeroed(&mut v, 3, 1).unwrap();
            assert_eq!(v.len(), 4);
            assert_eq!(&*v, &[(), (), (), ()]);
            drop(v);
        }

        #[test]
        fn test_new_box_zeroed() {
            assert_eq!(u64::new_box_zeroed(), Ok(Box::new(0)));
        }

        #[test]
        fn test_new_box_zeroed_array() {
            drop(<[u32; 0x1000]>::new_box_zeroed());
        }

        #[test]
        fn test_new_box_zeroed_zst() {
            // This test exists in order to exercise unsafe code, especially
            // when running under Miri.
            #[allow(clippy::unit_cmp)]
            {
                assert_eq!(<()>::new_box_zeroed(), Ok(Box::new(())));
            }
        }

        #[test]
        fn test_new_box_zeroed_with_elems() {
            let mut s: Box<[u64]> = <[u64]>::new_box_zeroed_with_elems(3).unwrap();
            assert_eq!(s.len(), 3);
            assert_eq!(&*s, &[0, 0, 0]);
            s[1] = 3;
            assert_eq!(&*s, &[0, 3, 0]);
        }

        #[test]
        fn test_new_box_zeroed_with_elems_nested_packed_dst() {
            let mut value = NestedPackedDstOuter::<[u8]>::new_box_zeroed_with_elems(0).unwrap();
            assert_eq!(mem::size_of_val(&*value), 10);

            // Exercise the whole-object write that exposed the former
            // under-allocation under Miri and AddressSanitizer.
            value.zero();
        }

        #[test]
        fn test_new_box_zeroed_with_elems_empty() {
            let s: Box<[u64]> = <[u64]>::new_box_zeroed_with_elems(0).unwrap();
            assert_eq!(s.len(), 0);
        }

        #[test]
        fn test_new_box_zeroed_with_elems_zst() {
            let mut s: Box<[()]> = <[()]>::new_box_zeroed_with_elems(3).unwrap();
            assert_eq!(s.len(), 3);
            assert!(s.get(10).is_none());
            // This test exists in order to exercise unsafe code, especially
            // when running under Miri.
            #[allow(clippy::unit_cmp)]
            {
                assert_eq!(s[1], ());
            }
            s[2] = ();
        }

        #[test]
        fn test_new_box_zeroed_with_elems_zst_empty() {
            let s: Box<[()]> = <[()]>::new_box_zeroed_with_elems(0).unwrap();
            assert_eq!(s.len(), 0);
        }

        #[test]
        fn new_box_zeroed_with_elems_errors() {
            assert_eq!(<[u16]>::new_box_zeroed_with_elems(usize::MAX), Err(AllocError));

            let max = <usize as core::convert::TryFrom<_>>::try_from(isize::MAX).unwrap();
            assert_eq!(
                <[u16]>::new_box_zeroed_with_elems((max / mem::size_of::<u16>()) + 1),
                Err(AllocError)
            );
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_from_bytes() {
        let val = 0u32;
        let bytes = val.as_bytes();

        assert!(u32::ref_from(bytes).is_some());
        // mut_from needs mut bytes
        let mut val = 0u32;
        let mut_bytes = val.as_mut_bytes();
        assert!(u32::mut_from(mut_bytes).is_some());

        assert!(u32::read_from(bytes).is_some());

        let (slc, rest) = <u32>::slice_from_prefix(bytes, 0).unwrap();
        assert!(slc.is_empty());
        assert_eq!(rest.len(), 4);

        let (rest, slc) = <u32>::slice_from_suffix(bytes, 0).unwrap();
        assert!(slc.is_empty());
        assert_eq!(rest.len(), 4);

        let (slc, rest) = <u32>::mut_slice_from_prefix(mut_bytes, 0).unwrap();
        assert!(slc.is_empty());
        assert_eq!(rest.len(), 4);

        let (rest, slc) = <u32>::mut_slice_from_suffix(mut_bytes, 0).unwrap();
        assert!(slc.is_empty());
        assert_eq!(rest.len(), 4);
    }

    #[test]
    fn test_ref_from_prefix_suffix_typed_source() {
        #[derive(IntoBytes, Immutable)]
        #[repr(transparent)]
        struct Source<T: ?Sized>(T);

        fn check<S: IntoBytes + Immutable + ?Sized>(source: &S) {
            let bytes = source.as_bytes();
            assert_eq!(bytes.len(), 4);

            for cast_type in [CastType::Prefix, CastType::Suffix] {
                let (expected, expected_rest) = match cast_type {
                    CastType::Prefix => (&bytes[..3], &bytes[3..]),
                    CastType::Suffix => (&bytes[1..], &bytes[..1]),
                };
                for meta in [None, Some(1)] {
                    let (value, rest) =
                        ref_from_prefix_suffix::<_, [[u8; 3]]>(source, meta, cast_type).unwrap();
                    assert_eq!(value, &[expected]);
                    assert!(ptr::eq(value.as_bytes(), expected));
                    assert!(ptr::eq(rest, expected_rest));
                }

                let err =
                    ref_from_prefix_suffix::<_, [u8; 5]>(source, None, cast_type).unwrap_err();
                assert!(matches!(err, CastError::Size(_)));
                assert!(ptr::eq(err.into_src(), source));

                let err =
                    ref_from_prefix_suffix::<_, [u8]>(source, Some(5), cast_type).unwrap_err();
                assert!(matches!(err, CastError::Size(_)));
                assert!(ptr::eq(err.into_src(), source));
            }
        }

        check(&Source([0x0102u16, 0x0304]));
        let source = Source([false, true, false, true]);
        let source: &Source<[bool]> = &source;
        check(source);
    }

    #[test]
    fn test_ref_from_prefix_suffix_typed_source_alignment_error() {
        let storage = Align::<[bool; 9], AU64>::new([false; 9]);
        for cast_type in [CastType::Prefix, CastType::Suffix] {
            let source = match cast_type {
                CastType::Prefix => &storage.t[1..],
                CastType::Suffix => &storage.t[..],
            };
            let err = ref_from_prefix_suffix::<_, AU64>(source, None, cast_type).unwrap_err();
            assert!(matches!(err, CastError::Alignment(_)));
            let recovered: &[bool] = err.into_src();
            assert!(ptr::eq(recovered, source));

            let err = try_ref_from_prefix_suffix::<_, AU64>(source, cast_type, None).unwrap_err();
            assert!(matches!(err, TryCastError::Alignment(_)));
            let recovered: &[bool] = err.into_src();
            assert!(ptr::eq(recovered, source));
        }
    }

    #[test]
    fn test_try_ref_from_prefix_suffix_typed_source() {
        #[derive(IntoBytes, Immutable)]
        #[repr(transparent)]
        struct Source<T: ?Sized>(T);

        fn check<S: IntoBytes + Immutable + ?Sized>(source: &S) {
            let bytes = source.as_bytes();
            assert_eq!(bytes, [0, 1, 0, 1]);

            for cast_type in [CastType::Prefix, CastType::Suffix] {
                let (expected, expected_rest) = match cast_type {
                    CastType::Prefix => (&bytes[..3], &bytes[3..]),
                    CastType::Suffix => (&bytes[1..], &bytes[..1]),
                };
                for meta in [None, Some(1)] {
                    let (value, rest) =
                        try_ref_from_prefix_suffix::<_, [[bool; 3]]>(source, cast_type, meta)
                            .unwrap();
                    assert_eq!(value.as_bytes(), expected);
                    assert!(ptr::eq(value.as_bytes(), expected));
                    assert!(ptr::eq(rest, expected_rest));
                }

                let err = try_ref_from_prefix_suffix::<_, [bool; 5]>(source, cast_type, None)
                    .unwrap_err();
                assert!(matches!(err, TryCastError::Size(_)));
                assert!(ptr::eq(err.into_src(), source));

                let err = try_ref_from_prefix_suffix::<_, [bool]>(source, cast_type, Some(5))
                    .unwrap_err();
                assert!(matches!(err, TryCastError::Size(_)));
                assert!(ptr::eq(err.into_src(), source));
            }
        }

        check(&Source([u16::from_ne_bytes([0, 1]); 2]));
        let source = Source([false, true, false, true]);
        let source: &Source<[bool]> = &source;
        check(source);
    }

    #[test]
    fn test_try_ref_from_prefix_suffix_typed_source_validity_error() {
        fn check<S: IntoBytes + Immutable + ?Sized>(source: &S) {
            for cast_type in [CastType::Prefix, CastType::Suffix] {
                let err =
                    try_ref_from_prefix_suffix::<_, bool>(source, cast_type, None).unwrap_err();
                assert!(matches!(err, TryCastError::Validity(_)));
                assert!(ptr::eq(err.into_src(), source));

                for meta in [None, Some(1)] {
                    let err = try_ref_from_prefix_suffix::<_, [bool]>(source, cast_type, meta)
                        .unwrap_err();
                    assert!(matches!(err, TryCastError::Validity(_)));
                    assert!(ptr::eq(err.into_src(), source));
                }
            }
        }

        let source = [u16::from_ne_bytes([2, 2]); 2];
        check(&source);
        check(&source[..]);
        check(source.as_bytes());
    }

    #[test]
    fn test_try_ref_from_prefix_suffix() {
        use crate::util::testutil::Align;
        let bytes = &Align::<[u8; 4], u32>::new([0u8; 4]).t[..];
        let (r, rest): (&u32, &[u8]) = u32::try_ref_from_prefix(bytes).unwrap();
        assert_eq!(*r, 0);
        assert_eq!(rest.len(), 0);

        let (rest, r): (&[u8], &u32) = u32::try_ref_from_suffix(bytes).unwrap();
        assert_eq!(*r, 0);
        assert_eq!(rest.len(), 0);
    }

    #[test]
    fn test_raw_dangling() {
        use crate::util::AsAddress;
        let ptr: NonNull<u32> = u32::raw_dangling();
        assert_eq!(AsAddress::addr(ptr), 1);

        let ptr: NonNull<[u32]> = <[u32]>::raw_dangling();
        assert_eq!(AsAddress::addr(ptr), 1);
    }

    #[test]
    fn test_try_ref_from_prefix_with_elems() {
        use crate::util::testutil::Align;
        let bytes = &Align::<[u8; 8], u32>::new([0u8; 8]).t[..];
        let (r, rest): (&[u32], &[u8]) = <[u32]>::try_ref_from_prefix_with_elems(bytes, 2).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(rest.len(), 0);
    }

    #[test]
    fn test_try_ref_from_suffix_with_elems() {
        use crate::util::testutil::Align;
        let bytes = &Align::<[u8; 8], u32>::new([0u8; 8]).t[..];
        let (rest, r): (&[u8], &[u32]) = <[u32]>::try_ref_from_suffix_with_elems(bytes, 2).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(rest.len(), 0);
    }
}
