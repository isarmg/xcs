#[cfg(test)]
mod tests {
    use core::mem::{self, MaybeUninit};

    use super::*;
    #[allow(unused)] // Needed on our MSRV, but considered unused on later toolchains.
    use crate::util::AsAddress;
    use crate::{pointer::BecauseImmutable, util::testutil::AU64, FromBytes, Immutable};

    #[test]
    fn test_project_tag_downgrades_aliasing() {
        #[allow(dead_code)]
        #[derive(zerocopy_derive::Project)]
        #[repr(u8)]
        enum Enum {
            Variant(u8),
        }

        let mut value = Enum::Variant(0);
        let _: Ptr<
            '_,
            <Enum as crate::HasTag<crate::project_clients::ProjectDerive>>::Tag,
            (Shared, Aligned, Safe),
        > = Ptr::from_mut(&mut value).project_tag::<crate::project_clients::ProjectDerive>();
    }

    mod test_ptr_try_cast_into_soundness {
        use super::*;

        // This test is designed so that if `Ptr::try_cast_into_xxx` are
        // buggy, it will manifest as unsoundness that Miri can detect.

        // - If `size_of::<T>() == 0`, `N == 4`
        // - Else, `N == 4 * size_of::<T>()`
        //
        // Each test will be run for each metadata in `metas`.
        fn test<T, I, const N: usize>(metas: I)
        where
            T: ?Sized + KnownLayout + Immutable + FromBytes,
            I: IntoIterator<Item = Option<T::PointerMetadata>> + Clone,
        {
            let mut bytes = [MaybeUninit::<u8>::uninit(); N];
            let initialized = [MaybeUninit::new(0u8); N];
            for start in 0..=bytes.len() {
                for end in start..=bytes.len() {
                    // Set all bytes to uninitialized other than those in
                    // the range we're going to pass to `try_cast_from`.
                    // This allows Miri to detect out-of-bounds reads
                    // because they read uninitialized memory. Without this,
                    // some out-of-bounds reads would still be in-bounds of
                    // `bytes`, and so might spuriously be accepted.
                    bytes = [MaybeUninit::<u8>::uninit(); N];
                    let bytes = &mut bytes[start..end];
                    // Initialize only the byte range we're going to pass to
                    // `try_cast_from`.
                    bytes.copy_from_slice(&initialized[start..end]);

                    let bytes = {
                        let bytes: *const [MaybeUninit<u8>] = bytes;
                        #[allow(clippy::as_conversions)]
                        let bytes = bytes as *const [u8];
                        // SAFETY: We just initialized these bytes to valid
                        // `u8`s.
                        unsafe { &*bytes }
                    };

                    // SAFETY: The bytes in `slf` must be initialized.
                    unsafe fn validate_and_get_len<
                        T: ?Sized + KnownLayout + FromBytes + Immutable,
                    >(
                        slf: Ptr<'_, T, (Shared, Aligned, Initialized)>,
                    ) -> usize {
                        let t = slf.recall_validity().as_ref();

                        let bytes = {
                            let len = mem::size_of_val(t);
                            let t: *const T = t;
                            // SAFETY:
                            // - We know `t`'s bytes are all initialized
                            //   because we just read it from `slf`, which
                            //   points to an initialized range of bytes. If
                            //   there's a bug and this doesn't hold, then
                            //   that's exactly what we're hoping Miri will
                            //   catch!
                            // - Since `T: FromBytes`, `T` doesn't contain
                            //   any `UnsafeCell`s, so it's okay for `t: T`
                            //   and a `&[u8]` to the same memory to be
                            //   alive concurrently.
                            unsafe { core::slice::from_raw_parts(t.cast::<u8>(), len) }
                        };

                        // This assertion ensures that `t`'s bytes are read
                        // and compared to another value, which in turn
                        // ensures that Miri gets a chance to notice if any
                        // of `t`'s bytes are uninitialized, which they
                        // shouldn't be (see the comment above).
                        assert_eq!(bytes, vec![0u8; bytes.len()]);

                        mem::size_of_val(t)
                    }

                    for meta in metas.clone().into_iter() {
                        for cast_type in [CastType::Prefix, CastType::Suffix] {
                            if let Ok((slf, remaining)) = Ptr::from_ref(bytes)
                                .try_cast_into::<T, BecauseImmutable>(cast_type, meta)
                            {
                                // SAFETY: All bytes in `bytes` have been
                                // initialized.
                                let len = unsafe { validate_and_get_len(slf) };
                                assert_eq!(remaining.len(), bytes.len() - len);
                                #[allow(unstable_name_collisions)]
                                let bytes_addr = bytes.as_ptr().addr();
                                #[allow(unstable_name_collisions)]
                                let remaining_addr = remaining.as_inner().as_ptr().addr();
                                match cast_type {
                                    CastType::Prefix => {
                                        assert_eq!(remaining_addr, bytes_addr + len)
                                    }
                                    CastType::Suffix => assert_eq!(remaining_addr, bytes_addr),
                                }

                                if let Some(want) = meta {
                                    let got =
                                        KnownLayout::pointer_to_metadata(slf.as_inner().as_ptr());
                                    assert_eq!(got, want);
                                }
                            }
                        }

                        if let Ok(slf) = Ptr::from_ref(bytes)
                            .try_cast_into_no_leftover::<T, BecauseImmutable>(meta)
                        {
                            // SAFETY: All bytes in `bytes` have been
                            // initialized.
                            let len = unsafe { validate_and_get_len(slf) };
                            assert_eq!(len, bytes.len());

                            if let Some(want) = meta {
                                let got = KnownLayout::pointer_to_metadata(slf.as_inner().as_ptr());
                                assert_eq!(got, want);
                            }
                        }
                    }
                }
            }
        }

        #[derive(FromBytes, KnownLayout, Immutable)]
        #[repr(C)]
        struct SliceDst<T> {
            a: u8,
            trailing: [T],
        }

        // Each test case becomes its own `#[test]` function. We do this because
        // this test in particular takes far, far longer to execute under Miri
        // than all of our other tests combined. Previously, we had these
        // execute sequentially in a single test function. We run Miri tests in
        // parallel in CI, but this test being sequential meant that most of
        // that parallelism was wasted, as all other tests would finish in a
        // fraction of the total execution time, leaving this test to execute on
        // a single thread for the remainder of the test. By putting each test
        // case in its own function, we permit better use of available
        // parallelism.
        macro_rules! test {
            ($test_name:ident: $ty:ty) => {
                #[test]
                #[allow(non_snake_case)]
                fn $test_name() {
                    const S: usize = core::mem::size_of::<$ty>();
                    const N: usize = if S == 0 { 4 } else { S * 4 };
                    test::<$ty, _, N>([None]);

                    // If `$ty` is a ZST, then we can't pass `None` as the
                    // pointer metadata, or else computing the correct trailing
                    // slice length will panic.
                    if S == 0 {
                        test::<[$ty], _, N>([Some(0), Some(1), Some(2), Some(3)]);
                        test::<SliceDst<$ty>, _, N>([Some(0), Some(1), Some(2), Some(3)]);
                    } else {
                        test::<[$ty], _, N>([None, Some(0), Some(1), Some(2), Some(3)]);
                        test::<SliceDst<$ty>, _, N>([None, Some(0), Some(1), Some(2), Some(3)]);
                    }
                }
            };
            ($ty:ident) => {
                test!($ty: $ty);
            };
            ($($ty:ident),*) => { $(test!($ty);)* }
        }

        test!(empty_tuple: ());
        test!(u8, u16, u32, u64, usize, AU64);
        test!(i8, i16, i32, i64, isize);
        test!(f32, f64);
    }

    #[test]
    fn test_try_cast_into_explicit_count() {
        macro_rules! test {
            ($ty:ty, $bytes:expr, $elems:expr, $expect:expr) => {{
                let bytes = [0u8; $bytes];
                let ptr = Ptr::from_ref(&bytes[..]);
                let res =
                    ptr.try_cast_into::<$ty, BecauseImmutable>(CastType::Prefix, Some($elems));
                if let Some(expect) = $expect {
                    let (ptr, _) = res.unwrap();
                    assert_eq!(KnownLayout::pointer_to_metadata(ptr.as_inner().as_ptr()), expect);
                } else {
                    let _ = res.unwrap_err();
                }
            }};
        }

        #[derive(KnownLayout, Immutable)]
        #[repr(C)]
        struct ZstDst {
            u: [u8; 8],
            slc: [()],
        }

        test!(ZstDst, 8, 0, Some(0));
        test!(ZstDst, 7, 0, None);

        test!(ZstDst, 8, usize::MAX, Some(usize::MAX));
        test!(ZstDst, 7, usize::MAX, None);

        #[derive(KnownLayout, Immutable)]
        #[repr(C)]
        struct Dst {
            u: [u8; 8],
            slc: [u8],
        }

        test!(Dst, 8, 0, Some(0));
        test!(Dst, 7, 0, None);

        test!(Dst, 9, 1, Some(1));
        test!(Dst, 8, 1, None);

        // If we didn't properly check for overflow, this would cause the
        // metadata to overflow to 0, and thus the cast would spuriously
        // succeed.
        test!(Dst, 8, usize::MAX - 8 + 1, None);
    }

    #[test]
    fn test_try_cast_into_no_leftover_restores_original_slice() {
        let bytes = [0u8; 4];
        let ptr = Ptr::from_ref(&bytes[..]);
        let res = ptr.try_cast_into_no_leftover::<[u8; 2], BecauseImmutable>(None);
        match res {
            Ok(_) => panic!("should have failed due to leftover bytes"),
            Err(CastError::Size(e)) => {
                assert_eq!(e.into_src().len(), 4, "Should return original slice length");
            }
            Err(e) => panic!("wrong error type: {:?}", e),
        }
    }

    #[test]
    fn test_iter_exclusive_yields_disjoint_ptrs() {
        let mut arr = [0u8, 1, 2, 3];

        {
            let mut iter = Ptr::from_mut(&mut arr[..]).iter();
            let first = iter.next().unwrap().as_mut();
            let second = iter.next().unwrap().as_mut();

            *first = 10;
            *second = 20;
            *first = 30;
        }

        assert_eq!(arr, [30, 20, 2, 3]);
    }
}
