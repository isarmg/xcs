#[cfg(test)]
#[allow(clippy::assertions_on_result_states)]
mod tests {
    use core::convert::TryInto as _;

    use super::*;
    use crate::util::testutil::*;

    #[test]
    fn test_mut_slice_into_ref() {
        // Prior to #1260/#1299, calling `into_ref` on a `&mut [u8]`-backed
        // `Ref` was not supported.
        let mut buf = [0u8];
        let r = Ref::<&mut [u8], u8>::from_bytes(&mut buf).unwrap();
        assert_eq!(Ref::into_ref(r), &0);
    }

    #[test]
    fn test_address() {
        // Test that the `Deref` and `DerefMut` implementations return a
        // reference which points to the right region of memory.

        let buf = [0];
        let r = Ref::<_, u8>::from_bytes(&buf[..]).unwrap();
        let buf_ptr = buf.as_ptr();
        let deref_ptr: *const u8 = r.deref();
        assert_eq!(buf_ptr, deref_ptr);

        let buf = [0];
        let r = Ref::<_, [u8]>::from_bytes(&buf[..]).unwrap();
        let buf_ptr = buf.as_ptr();
        let deref_ptr = r.deref().as_ptr();
        assert_eq!(buf_ptr, deref_ptr);
    }

    #[test]
    #[allow(clippy::clone_on_copy)] // Exercise `Ref`'s manual `Clone` impl.
    fn test_with_elems_preserves_ambiguous_metadata() {
        #[derive(FromBytes, Immutable, KnownLayout)]
        #[repr(C, align(8))]
        struct Frame {
            header: [u8; 8],
            body: [u8],
        }

        #[derive(FromBytes, Immutable, KnownLayout)]
        #[repr(C, align(8))]
        struct OddHeader {
            header: [u8; 7],
            body: [u8],
        }

        // Dynamic trailing padding makes these pairs of distinct element
        // counts produce the same total byte size. Thus, the byte length alone
        // cannot recover the count supplied to a `*_with_elems` constructor.
        assert_eq!(Frame::size_for_metadata(1), Some(16));
        assert_eq!(Frame::size_for_metadata(8), Some(16));
        assert_eq!(OddHeader::size_for_metadata(0), Some(8));
        assert_eq!(OddHeader::size_for_metadata(1), Some(8));

        let bytes = Align::<[u8; 32], AU64>::default();

        let whole = Ref::<_, Frame>::from_bytes_with_elems(&bytes.t[..16], 1).unwrap();
        assert_eq!(whole.body.len(), 1);
        assert_eq!(Ref::bytes(&whole).len(), 16);
        assert_eq!(whole.clone().body.len(), 1);
        let copied = whole;
        assert_eq!(copied.body.len(), 1);
        assert_eq!(Ref::into_ref(whole).body.len(), 1);

        let (prefix, rest) = Ref::<_, Frame>::from_prefix_with_elems(&bytes.t[..], 1).unwrap();
        assert_eq!(prefix.body.len(), 1);
        assert_eq!(rest.len(), 16);

        let (before, suffix) = Ref::<_, Frame>::from_suffix_with_elems(&bytes.t[..24], 1).unwrap();
        assert_eq!(before.len(), 8);
        assert_eq!(suffix.body.len(), 1);

        // Zero is real metadata, not a sentinel for "infer from the bytes".
        let empty = Ref::<_, OddHeader>::from_bytes_with_elems(&bytes.t[..8], 0).unwrap();
        assert!(empty.body.is_empty());
        assert!(Ref::into_ref(empty).body.is_empty());

        // The implicit constructors still choose the largest fitting count,
        // and retain the metadata selected during their validating cast.
        let inferred = Ref::<_, Frame>::from_bytes(&bytes.t[..16]).unwrap();
        assert_eq!(inferred.body.len(), 8);
        let (inferred, rest) = Ref::<_, Frame>::from_prefix(&bytes.t[..16]).unwrap();
        assert_eq!(inferred.body.len(), 8);
        assert!(rest.is_empty());
        let (before, inferred) = Ref::<_, Frame>::from_suffix(&bytes.t[..16]).unwrap();
        assert!(before.is_empty());
        assert_eq!(inferred.body.len(), 8);
    }

    // Verify that values written to a `Ref` are properly shared between the
    // typed and untyped representations, that reads via `deref` and `read`
    // behave the same, and that writes via `deref_mut` and `write` behave the
    // same.
    fn test_new_helper(mut r: Ref<&mut [u8], AU64>) {
        // assert that the value starts at 0
        assert_eq!(*r, AU64(0));
        assert_eq!(Ref::read(&r), AU64(0));

        // Assert that values written to the typed value are reflected in the
        // byte slice.
        const VAL1: AU64 = AU64(0xFF00FF00FF00FF00);
        *r = VAL1;
        assert_eq!(Ref::bytes(&r), &VAL1.to_bytes());
        *r = AU64(0);
        Ref::write(&mut r, VAL1);
        assert_eq!(Ref::bytes(&r), &VAL1.to_bytes());

        // Assert that values written to the byte slice are reflected in the
        // typed value.
        const VAL2: AU64 = AU64(!VAL1.0); // different from `VAL1`
        Ref::bytes_mut(&mut r).copy_from_slice(&VAL2.to_bytes()[..]);
        assert_eq!(*r, VAL2);
        assert_eq!(Ref::read(&r), VAL2);
    }

    // Verify that values written to a `Ref` are properly shared between the
    // typed and untyped representations; pass a value with `typed_len` `AU64`s
    // backed by an array of `typed_len * 8` bytes.
    fn test_new_helper_slice(mut r: Ref<&mut [u8], [AU64]>, typed_len: usize) {
        // Assert that the value starts out zeroed.
        assert_eq!(&*r, vec![AU64(0); typed_len].as_slice());

        // Check the backing storage is the exact same slice.
        let untyped_len = typed_len * 8;
        assert_eq!(Ref::bytes(&r).len(), untyped_len);
        assert_eq!(Ref::bytes(&r).as_ptr(), r.as_ptr().cast::<u8>());

        // Assert that values written to the typed value are reflected in the
        // byte slice.
        const VAL1: AU64 = AU64(0xFF00FF00FF00FF00);
        for typed in &mut *r {
            *typed = VAL1;
        }
        assert_eq!(Ref::bytes(&r), VAL1.0.to_ne_bytes().repeat(typed_len).as_slice());

        // Assert that values written to the byte slice are reflected in the
        // typed value.
        const VAL2: AU64 = AU64(!VAL1.0); // different from VAL1
        Ref::bytes_mut(&mut r).copy_from_slice(&VAL2.0.to_ne_bytes().repeat(typed_len));
        assert!(r.iter().copied().all(|x| x == VAL2));
    }

    #[test]
    fn test_new_aligned_sized() {
        // Test that a properly-aligned, properly-sized buffer works for new,
        // new_from_prefix, and new_from_suffix, and that new_from_prefix and
        // new_from_suffix return empty slices. Test that a properly-aligned
        // buffer whose length is a multiple of the element size works for
        // new_slice.

        // A buffer with an alignment of 8.
        let mut buf = Align::<[u8; 8], AU64>::default();
        // `buf.t` should be aligned to 8, so this should always succeed.
        test_new_helper(Ref::<_, AU64>::from_bytes(&mut buf.t[..]).unwrap());
        {
            // In a block so that `r` and `suffix` don't live too long.
            buf.set_default();
            let (r, suffix) = Ref::<_, AU64>::from_prefix(&mut buf.t[..]).unwrap();
            assert!(suffix.is_empty());
            test_new_helper(r);
        }
        {
            buf.set_default();
            let (prefix, r) = Ref::<_, AU64>::from_suffix(&mut buf.t[..]).unwrap();
            assert!(prefix.is_empty());
            test_new_helper(r);
        }

        // A buffer with alignment 8 and length 24. We choose this length very
        // intentionally: if we instead used length 16, then the prefix and
        // suffix lengths would be identical. In the past, we used length 16,
        // which resulted in this test failing to discover the bug uncovered in
        // #506.
        let mut buf = Align::<[u8; 24], AU64>::default();
        // `buf.t` should be aligned to 8 and have a length which is a multiple
        // of `size_of::<AU64>()`, so this should always succeed.
        test_new_helper_slice(Ref::<_, [AU64]>::from_bytes(&mut buf.t[..]).unwrap(), 3);
        buf.set_default();
        let r = Ref::<_, [AU64]>::from_bytes_with_elems(&mut buf.t[..], 3).unwrap();
        test_new_helper_slice(r, 3);

        let ascending: [u8; 24] = (0..24).collect::<Vec<_>>().try_into().unwrap();
        // 16 ascending bytes followed by 8 zeros.
        let mut ascending_prefix = ascending;
        ascending_prefix[16..].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        // 8 zeros followed by 16 ascending bytes.
        let mut ascending_suffix = ascending;
        ascending_suffix[..8].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);
        {
            buf.t = ascending_suffix;
            let (r, suffix) = Ref::<_, [AU64]>::from_prefix_with_elems(&mut buf.t[..], 1).unwrap();
            assert_eq!(suffix, &ascending[8..]);
            test_new_helper_slice(r, 1);
        }
        {
            buf.t = ascending_prefix;
            let (prefix, r) = Ref::<_, [AU64]>::from_suffix_with_elems(&mut buf.t[..], 1).unwrap();
            assert_eq!(prefix, &ascending[..16]);
            test_new_helper_slice(r, 1);
        }
    }

    #[test]
    fn test_new_oversized() {
        // Test that a properly-aligned, overly-sized buffer works for
        // `new_from_prefix` and `new_from_suffix`, and that they return the
        // remainder and prefix of the slice respectively.

        let mut buf = Align::<[u8; 16], AU64>::default();
        {
            // In a block so that `r` and `suffix` don't live too long. `buf.t`
            // should be aligned to 8, so this should always succeed.
            let (r, suffix) = Ref::<_, AU64>::from_prefix(&mut buf.t[..]).unwrap();
            assert_eq!(suffix.len(), 8);
            test_new_helper(r);
        }
        {
            buf.set_default();
            // `buf.t` should be aligned to 8, so this should always succeed.
            let (prefix, r) = Ref::<_, AU64>::from_suffix(&mut buf.t[..]).unwrap();
            assert_eq!(prefix.len(), 8);
            test_new_helper(r);
        }
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_new_error() {
        // Fail because the buffer is too large.

        // A buffer with an alignment of 8.
        let buf = Align::<[u8; 16], AU64>::default();
        // `buf.t` should be aligned to 8, so only the length check should fail.
        assert!(Ref::<_, AU64>::from_bytes(&buf.t[..]).is_err());

        // Fail because the buffer is too small.

        // A buffer with an alignment of 8.
        let buf = Align::<[u8; 4], AU64>::default();
        // `buf.t` should be aligned to 8, so only the length check should fail.
        assert!(Ref::<_, AU64>::from_bytes(&buf.t[..]).is_err());
        assert!(Ref::<_, AU64>::from_prefix(&buf.t[..]).is_err());
        assert!(Ref::<_, AU64>::from_suffix(&buf.t[..]).is_err());

        // Fail because the length is not a multiple of the element size.

        let buf = Align::<[u8; 12], AU64>::default();
        // `buf.t` has length 12, but element size is 8.
        assert!(Ref::<_, [AU64]>::from_bytes(&buf.t[..]).is_err());

        // Fail because the buffer is too short.
        let buf = Align::<[u8; 12], AU64>::default();
        // `buf.t` has length 12, but the element size is 8 (and we're expecting
        // two of them). For each function, we test with a length that would
        // cause the size to overflow `usize`, and with a normal length that
        // will fail thanks to the buffer being too short; these are different
        // error paths, and while the error types are the same, the distinction
        // shows up in code coverage metrics.
        let n = (usize::MAX / mem::size_of::<AU64>()) + 1;
        assert!(Ref::<_, [AU64]>::from_bytes_with_elems(&buf.t[..], n).is_err());
        assert!(Ref::<_, [AU64]>::from_bytes_with_elems(&buf.t[..], 2).is_err());
        assert!(Ref::<_, [AU64]>::from_prefix_with_elems(&buf.t[..], n).is_err());
        assert!(Ref::<_, [AU64]>::from_prefix_with_elems(&buf.t[..], 2).is_err());
        assert!(Ref::<_, [AU64]>::from_suffix_with_elems(&buf.t[..], n).is_err());
        assert!(Ref::<_, [AU64]>::from_suffix_with_elems(&buf.t[..], 2).is_err());

        // Fail because the alignment is insufficient.

        // A buffer with an alignment of 8. An odd buffer size is chosen so that
        // the last byte of the buffer has odd alignment.
        let buf = Align::<[u8; 13], AU64>::default();
        // Slicing from 1, we get a buffer with size 12 (so the length check
        // should succeed) but an alignment of only 1, which is insufficient.
        assert!(Ref::<_, AU64>::from_bytes(&buf.t[1..]).is_err());
        assert!(Ref::<_, AU64>::from_prefix(&buf.t[1..]).is_err());
        assert!(Ref::<_, [AU64]>::from_bytes(&buf.t[1..]).is_err());
        assert!(Ref::<_, [AU64]>::from_bytes_with_elems(&buf.t[1..], 1).is_err());
        assert!(Ref::<_, [AU64]>::from_prefix_with_elems(&buf.t[1..], 1).is_err());
        assert!(Ref::<_, [AU64]>::from_suffix_with_elems(&buf.t[1..], 1).is_err());
        // Slicing is unnecessary here because `new_from_suffix` uses the suffix
        // of the slice, which has odd alignment.
        assert!(Ref::<_, AU64>::from_suffix(&buf.t[..]).is_err());

        // Fail due to arithmetic overflow.

        let buf = Align::<[u8; 16], AU64>::default();
        let unreasonable_len = usize::MAX / mem::size_of::<AU64>() + 1;
        assert!(Ref::<_, [AU64]>::from_prefix_with_elems(&buf.t[..], unreasonable_len).is_err());
        assert!(Ref::<_, [AU64]>::from_suffix_with_elems(&buf.t[..], unreasonable_len).is_err());
    }

    #[test]
    #[allow(unstable_name_collisions)]
    #[allow(clippy::as_conversions)]
    fn test_into_ref_mut() {
        #[allow(unused)]
        use crate::util::AsAddress as _;

        let mut buf = Align::<[u8; 8], u64>::default();
        let r = Ref::<_, u64>::from_bytes(&buf.t[..]).unwrap();
        let rf = Ref::into_ref(r);
        assert_eq!(rf, &0u64);
        let buf_addr = (&buf.t as *const [u8; 8]).addr();
        assert_eq!((rf as *const u64).addr(), buf_addr);

        let r = Ref::<_, u64>::from_bytes(&mut buf.t[..]).unwrap();
        let rf = Ref::into_mut(r);
        assert_eq!(rf, &mut 0u64);
        assert_eq!((rf as *mut u64).addr(), buf_addr);

        *rf = u64::MAX;
        assert_eq!(buf.t, [0xFF; 8]);

        // Exercise the unsized `into_mut` route, which must use the metadata
        // retained by the constructor rather than infer fresh metadata.
        let mut buf = Align::<[u8; 8], u16>::default();
        let r = Ref::<_, [u16]>::from_bytes_with_elems(&mut buf.t[..], 4).unwrap();
        let rf = Ref::into_mut(r);
        assert_eq!(rf.len(), 4);
    }

    #[test]
    fn test_display_debug() {
        let buf = Align::<[u8; 8], u64>::default();
        let r = Ref::<_, u64>::from_bytes(&buf.t[..]).unwrap();
        assert_eq!(format!("{}", r), "0");
        assert_eq!(format!("{:?}", r), "Ref(0)");

        let buf = Align::<[u8; 8], u64>::default();
        let r = Ref::<_, [u64]>::from_bytes(&buf.t[..]).unwrap();
        assert_eq!(format!("{:?}", r), "Ref([0])");
    }

    #[test]
    fn test_eq() {
        let buf1 = 0_u64;
        let r1 = Ref::<_, u64>::from_bytes(buf1.as_bytes()).unwrap();
        let buf2 = 0_u64;
        let r2 = Ref::<_, u64>::from_bytes(buf2.as_bytes()).unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_ne() {
        let buf1 = 0_u64;
        let r1 = Ref::<_, u64>::from_bytes(buf1.as_bytes()).unwrap();
        let buf2 = 1_u64;
        let r2 = Ref::<_, u64>::from_bytes(buf2.as_bytes()).unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_ord() {
        let buf1 = 0_u64;
        let r1 = Ref::<_, u64>::from_bytes(buf1.as_bytes()).unwrap();
        let buf2 = 1_u64;
        let r2 = Ref::<_, u64>::from_bytes(buf2.as_bytes()).unwrap();
        assert!(r1 < r2);
        assert_eq!(PartialOrd::partial_cmp(&r1, &r2), Some(Ordering::Less));
        assert_eq!(Ord::cmp(&r1, &r2), Ordering::Less);
    }
}
