#[cfg(test)]
mod tests {
    use core::panic::AssertUnwindSafe;

    use super::*;
    use crate::util::testutil::*;

    #[test]
    #[allow(clippy::clone_on_copy, clippy::non_canonical_clone_impl)]
    fn test_read_only_copy_clone() {
        // This type deliberately does not implement `Immutable`. Copying the
        // wrapper must not require it or invoke the inner `Clone` impl.
        #[derive(Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        struct PanickingClone(u16);

        impl Clone for PanickingClone {
            fn clone(&self) -> Self {
                panic!("ReadOnly must not call the inner Clone implementation")
            }
        }

        static_assertions::assert_not_impl_any!(
            ReadOnly<PanickingClone>: Debug, PartialEq, Eq, PartialOrd, Ord,
            Hash, AsRef<PanickingClone>, Borrow<PanickingClone>
        );
        static_assertions::assert_not_impl_any!(
            ReadOnly<Cell<u8>>: Copy, Clone, Deref, AsRef<Cell<u8>>, Borrow<Cell<u8>>
        );

        let value = ReadOnly::new(PanickingClone(42));
        let copy = value;
        let clone = value.clone();
        assert_eq!(ReadOnly::into_inner(copy).0, 42);
        assert_eq!(ReadOnly::into_inner(clone).0, 42);
    }

    #[test]
    fn test_read_only_mutable_access() {
        // Construction and exclusive access also support interior mutability.
        let mut value = ReadOnly::<Cell<u8>>::default();
        let inner: &mut Cell<u8> = value.as_mut();
        inner.set(42);
        assert_eq!(ReadOnly::into_inner(value).get(), 42);

        let value = ReadOnly::from(Cell::new(7));
        assert_eq!(ReadOnly::into_inner(value).get(), 7);
    }

    #[test]
    fn test_read_only_borrowed_lookup() {
        // Borrowed lookups require equality, hashing, and ordering to agree
        // between the wrapper and the inner value.
        let mut hash_map = std::collections::HashMap::new();
        hash_map.insert(ReadOnly::new(42u16), true);
        assert_eq!(hash_map.get(&42u16), Some(&true));
        assert_eq!(hash_map.get(&43u16), None);

        let mut tree = std::collections::BTreeMap::new();
        tree.insert(ReadOnly::new(42u16), true);
        assert_eq!(tree.get(&42u16), Some(&true));
        assert_eq!(tree.get(&43u16), None);

        // Preserve partial orders, including incomparable values.
        let nan = ReadOnly::new(f32::NAN);
        assert_eq!(nan.partial_cmp(&nan), None);
        assert_ne!(nan, nan);
    }

    #[test]
    fn test_read_only_unsized() {
        let bytes = [1u8, 2];
        let value: &ReadOnly<[u8]> = (&bytes[..]).into();
        let as_ref: &[u8] = value.as_ref();
        let borrowed: &[u8] = value.borrow();
        assert_eq!(as_ref, bytes);
        assert_eq!(borrowed, bytes);
        assert_eq!(value, value);
        assert_eq!(value.cmp(value), Ordering::Equal);

        let text: &ReadOnly<str> = "hello".into();
        assert_eq!(format!("{:.3}", text), "hel");
    }

    #[test]
    fn test_read_only_formatting() {
        let value = ReadOnly::new(42u16);
        // Check that formatting flags are forwarded as well as the value.
        assert_eq!(format!("{:04?}", value), "0042");
        assert_eq!(format!("{:04}", value), "0042");
        assert_eq!(format!("{:#010b}", value), "0b00101010");
        assert_eq!(format!("{:#06o}", value), "0o0052");
        assert_eq!(format!("{:#06x}", value), "0x002a");
        assert_eq!(format!("{:#06X}", value), "0x002A");
        assert_eq!(format!("{:.2e}", value), "4.20e1");
        assert_eq!(format!("{:.2E}", value), "4.20E1");
    }

    #[test]
    fn test_unalign() {
        // Test methods that don't depend on alignment.
        let mut u = Unalign::new(AU64(123));
        assert_eq!(u.get(), AU64(123));
        assert_eq!(u.into_inner(), AU64(123));
        assert_eq!(u.get_ptr(), <*const _>::cast::<AU64>(&u));
        assert_eq!(u.get_mut_ptr(), <*mut _>::cast::<AU64>(&mut u));
        u.set(AU64(321));
        assert_eq!(u.get(), AU64(321));

        // Test methods that depend on alignment (when alignment is satisfied).
        let mut u: Align<_, AU64> = Align::new(Unalign::new(AU64(123)));
        assert_eq!(u.t.try_deref().unwrap(), &AU64(123));
        assert_eq!(u.t.try_deref_mut().unwrap(), &mut AU64(123));
        // SAFETY: The `Align<_, AU64>` guarantees proper alignment.
        assert_eq!(unsafe { u.t.deref_unchecked() }, &AU64(123));
        // SAFETY: The `Align<_, AU64>` guarantees proper alignment.
        assert_eq!(unsafe { u.t.deref_mut_unchecked() }, &mut AU64(123));
        *u.t.try_deref_mut().unwrap() = AU64(321);
        assert_eq!(u.t.get(), AU64(321));

        // Test methods that depend on alignment (when alignment is not
        // satisfied).
        let mut u: ForceUnalign<_, AU64> = ForceUnalign::new(Unalign::new(AU64(123)));
        assert!(matches!(u.t.try_deref(), Err(AlignmentError { .. })));
        assert!(matches!(u.t.try_deref_mut(), Err(AlignmentError { .. })));

        // Test methods that depend on `T: Unaligned`.
        let mut u = Unalign::new(123u8);
        assert_eq!(u.try_deref(), Ok(&123));
        assert_eq!(u.try_deref_mut(), Ok(&mut 123));
        assert_eq!(u.deref(), &123);
        assert_eq!(u.deref_mut(), &mut 123);
        *u = 21;
        assert_eq!(u.get(), 21);

        // Test that some `Unalign` functions and methods are `const`.
        const _UNALIGN: Unalign<u64> = Unalign::new(0);
        const _UNALIGN_PTR: *const u64 = _UNALIGN.get_ptr();
        const _U64: u64 = _UNALIGN.into_inner();
        // Make sure all code is considered "used".
        //
        // FIXME(https://github.com/rust-lang/rust/issues/104084): Remove this
        // attribute.
        #[allow(dead_code)]
        const _: () = {
            let x: Align<_, AU64> = Align::new(Unalign::new(AU64(123)));
            // Make sure that `deref_unchecked` is `const`.
            //
            // SAFETY: The `Align<_, AU64>` guarantees proper alignment.
            let au64 = unsafe { x.t.deref_unchecked() };
            match au64 {
                AU64(123) => {}
                _ => const_unreachable!(),
            }
        };
    }

    #[test]
    fn test_unalign_update() {
        let mut u = Unalign::new(AU64(123));
        u.update(|a| a.0 += 1);
        assert_eq!(u.get(), AU64(124));

        // Test that, even if the callback panics, the original is still
        // correctly overwritten. Use a `Box` so that Miri is more likely to
        // catch any unsoundness (which would likely result in two `Box`es for
        // the same heap object, which is the sort of thing that Miri would
        // probably catch).
        let mut u = Unalign::new(Box::new(AU64(123)));
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            u.update(|a| {
                a.0 += 1;
                panic!();
            })
        }));
        assert!(res.is_err());
        assert_eq!(u.into_inner(), Box::new(AU64(124)));

        // Test the align_of::<T>() == 1 optimization.
        let mut u = Unalign::new([0u8, 1]);
        u.update(|a| a[0] += 1);
        assert_eq!(u.get(), [1u8, 1]);
    }

    #[test]
    fn test_unalign_copy_clone() {
        // Test that `Copy` and `Clone` do not cause soundness issues. This test
        // is mainly meant to exercise UB that would be caught by Miri.

        // `u.t` is definitely not validly-aligned for `AU64`'s alignment of 8.
        let u = ForceUnalign::<_, AU64>::new(Unalign::new(AU64(123)));
        #[allow(clippy::clone_on_copy)]
        let v = u.t.clone();
        let w = u.t;
        assert_eq!(u.t.get(), v.get());
        assert_eq!(u.t.get(), w.get());
        assert_eq!(v.get(), w.get());
    }

    #[test]
    fn test_unalign_trait_impls() {
        let zero = Unalign::new(0u8);
        let one = Unalign::new(1u8);

        assert!(zero < one);
        assert_eq!(PartialOrd::partial_cmp(&zero, &one), Some(Ordering::Less));
        assert_eq!(Ord::cmp(&zero, &one), Ordering::Less);

        assert_ne!(zero, one);
        assert_eq!(zero, zero);
        assert!(!PartialEq::eq(&zero, &one));
        assert!(PartialEq::eq(&zero, &zero));

        fn hash<T: Hash>(t: &T) -> u64 {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut h);
            h.finish()
        }

        assert_eq!(hash(&zero), hash(&0u8));
        assert_eq!(hash(&one), hash(&1u8));

        assert_eq!(format!("{:?}", zero), format!("{:?}", 0u8));
        assert_eq!(format!("{:?}", one), format!("{:?}", 1u8));
        assert_eq!(format!("{}", zero), format!("{}", 0u8));
        assert_eq!(format!("{}", one), format!("{}", 1u8));
    }

    #[test]
    #[allow(clippy::as_conversions)]
    fn test_maybe_uninit() {
        // int
        {
            let input = 42;
            let uninit = MaybeUninit::new(input);
            // SAFETY: `uninit` is in an initialized state
            let output = unsafe { uninit.assume_init() };
            assert_eq!(input, output);
        }

        // thin ref
        {
            let input = 42;
            let uninit = MaybeUninit::new(&input);
            // SAFETY: `uninit` is in an initialized state
            let output = unsafe { uninit.assume_init() };
            assert_eq!(&input as *const _, output as *const _);
            assert_eq!(input, *output);
        }

        // wide ref
        {
            let input = [1, 2, 3, 4];
            let uninit = MaybeUninit::new(&input[..]);
            // SAFETY: `uninit` is in an initialized state
            let output = unsafe { uninit.assume_init() };
            assert_eq!(&input[..] as *const _, output as *const _);
            assert_eq!(input, *output);
        }
    }
    #[test]
    fn test_maybe_uninit_uninit() {
        let _uninit = MaybeUninit::<u8>::uninit();
        // Cannot check value, but can check it compiles and runs
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_maybe_uninit_new_boxed_uninit() {
        let _boxed = MaybeUninit::<u8>::new_boxed_uninit(()).unwrap();
    }

    #[test]
    fn test_maybe_uninit_debug() {
        let uninit = MaybeUninit::<u8>::uninit();
        assert!(format!("{:?}", uninit).contains("MaybeUninit"));
    }
}
