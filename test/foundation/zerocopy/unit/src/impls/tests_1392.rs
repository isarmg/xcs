#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impls() {
        // A type that can supply test cases for testing `TryFromBytes::is_safe`.
        // All types passed to `assert_impls!` must implement this trait; that
        // macro uses it to generate runtime tests for `TryFromBytes` impls.
        //
        // All `T: FromBytes` types are provided with a blanket impl. Other
        // types must implement `TryFromBytesTestable` directly (ie using
        // `impl_try_from_bytes_testable!`).
        trait TryFromBytesTestable {
            fn with_passing_test_cases<F: Fn(Box<ReadOnly<Self>>)>(f: F);
            fn with_failing_test_cases<F: Fn(&mut [u8])>(f: F);
        }

        impl<T: FromBytes> TryFromBytesTestable for T {
            fn with_passing_test_cases<F: Fn(Box<ReadOnly<Self>>)>(f: F) {
                // Test with a zeroed value.
                f(ReadOnly::<Self>::new_box_zeroed().unwrap());

                let ffs = {
                    let mut t = ReadOnly::new(Self::new_zeroed());
                    let ptr: *mut T = ReadOnly::as_mut(&mut t);
                    // SAFETY: `T: FromBytes`
                    unsafe { ptr::write_bytes(ptr.cast::<u8>(), 0xFF, mem::size_of::<T>()) };
                    t
                };

                // Test with a value initialized with 0xFF.
                f(Box::new(ffs));
            }

            fn with_failing_test_cases<F: Fn(&mut [u8])>(_f: F) {}
        }

        macro_rules! impl_try_from_bytes_testable_for_null_pointer_optimization {
            ($($tys:ty),*) => {
                $(
                    impl TryFromBytesTestable for Option<$tys> {
                        fn with_passing_test_cases<F: Fn(Box<ReadOnly<Self>>)>(f: F) {
                            // Test with a zeroed value.
                            f(Box::new(ReadOnly::new(None)));
                        }

                        fn with_failing_test_cases<F: Fn(&mut [u8])>(f: F) {
                            for pos in 0..mem::size_of::<Self>() {
                                let mut bytes = [0u8; mem::size_of::<Self>()];
                                bytes[pos] = 0x01;
                                f(&mut bytes[..]);
                            }
                        }
                    }
                )*
            };
        }

        // Implements `TryFromBytesTestable`.
        macro_rules! impl_try_from_bytes_testable {
            // Base case for recursion (when the list of types has run out).
            (=> @success $($success_case:expr),* $(, @failure $($failure_case:expr),*)?) => {};
            // Implements for type(s) with no type parameters.
            ($ty:ty $(,$tys:ty)* => @success $($success_case:expr),* $(, @failure $($failure_case:expr),*)?) => {
                impl TryFromBytesTestable for $ty {
                    impl_try_from_bytes_testable!(
                        @methods     @success $($success_case),*
                                 $(, @failure $($failure_case),*)?
                    );
                }
                impl_try_from_bytes_testable!($($tys),* => @success $($success_case),* $(, @failure $($failure_case),*)?);
            };
            // Implements for multiple types with no type parameters.
            ($($($ty:ty),* => @success $($success_case:expr), * $(, @failure $($failure_case:expr),*)?;)*) => {
                $(
                    impl_try_from_bytes_testable!($($ty),* => @success $($success_case),* $(, @failure $($failure_case),*)*);
                )*
            };
            // Implements only the methods; caller must invoke this from inside
            // an impl block.
            (@methods @success $($success_case:expr),* $(, @failure $($failure_case:expr),*)?) => {
                fn with_passing_test_cases<F: Fn(Box<ReadOnly<Self>>)>(_f: F) {
                    $(
                        let bx = Box::<Self>::from($success_case);
                        let ro: Box<ReadOnly<_>> = {
                            let raw = Box::into_raw(bx);
                            // SAFETY: `ReadOnly<T>` has the same layout and bit
                            // validity as `T`.
                            #[allow(clippy::as_conversions)]
                            unsafe { Box::from_raw(raw as *mut _) }
                        };
                        _f(ro);
                    )*
                }

                fn with_failing_test_cases<F: Fn(&mut [u8])>(_f: F) {
                    $($(
                        let mut case = $failure_case;
                        _f(case.as_mut_bytes());
                    )*)?
                }
            };
        }

        impl_try_from_bytes_testable_for_null_pointer_optimization!(
            Box<UnsafeCell<NotZerocopy>>,
            &'static UnsafeCell<NotZerocopy>,
            &'static mut UnsafeCell<NotZerocopy>,
            NonNull<UnsafeCell<NotZerocopy>>,
            fn(),
            FnManyArgs,
            extern "C" fn(),
            ECFnManyArgs
        );

        macro_rules! bx {
            ($e:expr) => {
                Box::new($e)
            };
        }

        // Note that these impls are only for types which are not `FromBytes`.
        // `FromBytes` types are covered by a preceding blanket impl.
        impl_try_from_bytes_testable!(
            bool => @success true, false,
                    @failure 2u8, 3u8, 0xFFu8;
            char => @success '\u{0}', '\u{D7FF}', '\u{E000}', '\u{10FFFF}',
                    @failure 0xD800u32, 0xDFFFu32, 0x110000u32;
            str  => @success "", "hello", "❤️🧡💛💚💙💜",
                    @failure [0, 159, 146, 150];
            [u8] => @success vec![].into_boxed_slice(), vec![0, 1, 2].into_boxed_slice();
            NonZeroU8, NonZeroI8, NonZeroU16, NonZeroI16, NonZeroU32,
            NonZeroI32, NonZeroU64, NonZeroI64, NonZeroU128, NonZeroI128,
            NonZeroUsize, NonZeroIsize
                => @success Self::new(1).unwrap(),
                   // Doing this instead of `0` ensures that we always satisfy
                   // the size and alignment requirements of `Self` (whereas `0`
                   // may be any integer type with a different size or alignment
                   // than some `NonZeroXxx` types).
                   @failure Option::<Self>::None;
            [bool; 0] => @success [];
            [bool; 1]
                => @success [true], [false],
                   @failure [2u8], [3u8], [0xFFu8];
            [bool]
                => @success vec![true, false].into_boxed_slice(), vec![false, true].into_boxed_slice(),
                    @failure [2u8], [3u8], [0xFFu8], [0u8, 1u8, 2u8];
            Unalign<bool>
                => @success Unalign::new(false), Unalign::new(true),
                   @failure 2u8, 0xFFu8;
            ManuallyDrop<bool>
                => @success ManuallyDrop::new(false), ManuallyDrop::new(true),
                   @failure 2u8, 0xFFu8;
            ManuallyDrop<[u8]>
                => @success bx!(ManuallyDrop::new([])), bx!(ManuallyDrop::new([0u8])), bx!(ManuallyDrop::new([0u8, 1u8]));
            ManuallyDrop<[bool]>
                => @success bx!(ManuallyDrop::new([])), bx!(ManuallyDrop::new([false])), bx!(ManuallyDrop::new([false, true])),
                   @failure [2u8], [3u8], [0xFFu8], [0u8, 1u8, 2u8];
            ManuallyDrop<[UnsafeCell<u8>]>
                => @success bx!(ManuallyDrop::new([UnsafeCell::new(0)])), bx!(ManuallyDrop::new([UnsafeCell::new(0), UnsafeCell::new(1)]));
            ManuallyDrop<[UnsafeCell<bool>]>
                => @success bx!(ManuallyDrop::new([UnsafeCell::new(false)])), bx!(ManuallyDrop::new([UnsafeCell::new(false), UnsafeCell::new(true)])),
                @failure [2u8], [3u8], [0xFFu8], [0u8, 1u8, 2u8];
            Wrapping<bool>
                => @success Wrapping(false), Wrapping(true),
                    @failure 2u8, 0xFFu8;
            *const NotZerocopy
                => @success ptr::null::<NotZerocopy>(),
                   @failure [0x01; mem::size_of::<*const NotZerocopy>()];
            *mut NotZerocopy
                => @success ptr::null_mut::<NotZerocopy>(),
                   @failure [0x01; mem::size_of::<*mut NotZerocopy>()];
        );

        // Use the trick described in [1] to allow us to call methods
        // conditional on certain trait bounds.
        //
        // In all of these cases, methods return `Option<R>`, where `R` is the
        // return type of the method we're conditionally calling. The "real"
        // implementations (the ones defined in traits using `&self`) return
        // `Some`, and the default implementations (the ones defined as inherent
        // methods using `&mut self`) return `None`.
        //
        // [1] https://github.com/dtolnay/case-studies/blob/master/autoref-specialization/README.md
        mod autoref_trick {
            use super::*;

            pub(super) struct AutorefWrapper<T: ?Sized>(pub(super) PhantomData<T>);

            pub(super) trait TestIsSafeShared<T: ?Sized> {
                #[allow(clippy::needless_lifetimes)]
                fn test_is_safe_shared<'ptr>(&self, candidate: Maybe<'ptr, T>) -> Option<bool>;
            }

            impl<T: TryFromBytes + Immutable + ?Sized> TestIsSafeShared<T> for AutorefWrapper<T> {
                #[allow(clippy::needless_lifetimes)]
                fn test_is_safe_shared<'ptr>(&self, candidate: Maybe<'ptr, T>) -> Option<bool> {
                    Some(T::is_safe(candidate))
                }
            }

            pub(super) trait TestTryFromRef<T: ?Sized> {
                #[allow(clippy::needless_lifetimes)]
                fn test_try_from_ref<'bytes>(
                    &self,
                    bytes: &'bytes [u8],
                ) -> Option<Option<&'bytes T>>;
            }

            impl<T: TryFromBytes + Immutable + KnownLayout + ?Sized> TestTryFromRef<T> for AutorefWrapper<T> {
                #[allow(clippy::needless_lifetimes)]
                fn test_try_from_ref<'bytes>(
                    &self,
                    bytes: &'bytes [u8],
                ) -> Option<Option<&'bytes T>> {
                    Some(T::try_ref_from_bytes(bytes).ok())
                }
            }

            pub(super) trait TestTryFromMut<T: ?Sized> {
                #[allow(clippy::needless_lifetimes)]
                fn test_try_from_mut<'bytes>(
                    &self,
                    bytes: &'bytes mut [u8],
                ) -> Option<Option<&'bytes mut T>>;
            }

            impl<T: TryFromBytes + IntoBytes + KnownLayout + ?Sized> TestTryFromMut<T> for AutorefWrapper<T> {
                #[allow(clippy::needless_lifetimes)]
                fn test_try_from_mut<'bytes>(
                    &self,
                    bytes: &'bytes mut [u8],
                ) -> Option<Option<&'bytes mut T>> {
                    Some(T::try_mut_from_bytes(bytes).ok())
                }
            }

            pub(super) trait TestTryReadFrom<T> {
                fn test_try_read_from(&self, bytes: &[u8]) -> Option<Option<T>>;
            }

            impl<T: TryFromBytes> TestTryReadFrom<T> for AutorefWrapper<T> {
                fn test_try_read_from(&self, bytes: &[u8]) -> Option<Option<T>> {
                    Some(T::try_read_from_bytes(bytes).ok())
                }
            }

            pub(super) trait TestAsBytes<T: ?Sized> {
                #[allow(clippy::needless_lifetimes)]
                fn test_as_bytes<'slf, 't>(&'slf self, t: &'t ReadOnly<T>) -> Option<&'t [u8]>;
            }

            impl<T: IntoBytes + Immutable + ?Sized> TestAsBytes<T> for AutorefWrapper<T> {
                #[allow(clippy::needless_lifetimes)]
                fn test_as_bytes<'slf, 't>(&'slf self, t: &'t ReadOnly<T>) -> Option<&'t [u8]> {
                    Some(t.as_bytes())
                }
            }
        }

        use autoref_trick::*;

        // Asserts that `$ty` is one of a list of types which are allowed to not
        // provide a "real" implementation for `$fn_name`. Since the
        // `autoref_trick` machinery fails silently, this allows us to ensure
        // that the "default" impls are only being used for types which we
        // expect.
        //
        // Note that, since this is a runtime test, it is possible to have an
        // allowlist which is too restrictive if the function in question is
        // never called for a particular type. For example, if `as_bytes` is not
        // supported for a particular type, and so `test_as_bytes` returns
        // `None`, methods such as `test_try_from_ref` may never be called for
        // that type. As a result, it's possible that, for example, adding
        // `as_bytes` support for a type would cause other allowlist assertions
        // to fail. This means that allowlist assertion failures should not
        // automatically be taken as a sign of a bug.
        macro_rules! assert_on_allowlist {
            ($fn_name:ident($ty:ty) $(: $($tys:ty),*)?) => {{
                use core::any::TypeId;

                let allowlist: &[TypeId] = &[ $($(TypeId::of::<$tys>()),*)? ];
                let allowlist_names: &[&str] = &[ $($(stringify!($tys)),*)? ];

                let id = TypeId::of::<$ty>();
                assert!(allowlist.contains(&id), "{} is not on allowlist for {}: {:?}", stringify!($ty), stringify!($fn_name), allowlist_names);
            }};
        }

        // Asserts that `$ty` implements any `$trait` and doesn't implement any
        // `!$trait`. Note that all `$trait`s must come before any `!$trait`s.
        //
        // For `T: TryFromBytes`, uses `TryFromBytesTestable` to test success
        // and failure cases.
        macro_rules! assert_impls {
            ($ty:ty: TryFromBytes) => {
                // "Default" implementations that match the "real"
                // implementations defined in the `autoref_trick` module above.
                #[allow(unused, non_local_definitions)]
                impl AutorefWrapper<$ty> {
                    #[allow(clippy::needless_lifetimes)]
                    fn test_is_safe_shared<'ptr>(
                        &mut self,
                        candidate: Maybe<'ptr, $ty>,
                    ) -> Option<bool> {
                        assert_on_allowlist!(
                            test_is_safe_shared($ty):
                            ManuallyDrop<UnsafeCell<()>>,
                            ManuallyDrop<[UnsafeCell<u8>]>,
                            ManuallyDrop<[UnsafeCell<bool>]>,
                            CoreMaybeUninit<NotZerocopy>,
                            CoreMaybeUninit<UnsafeCell<()>>,
                            Wrapping<UnsafeCell<()>>
                        );

                        None
                    }

                    #[allow(clippy::needless_lifetimes)]
                    fn test_try_from_ref<'bytes>(&mut self, _bytes: &'bytes [u8]) -> Option<Option<&'bytes $ty>> {
                        assert_on_allowlist!(
                            test_try_from_ref($ty):
                            ManuallyDrop<[UnsafeCell<bool>]>
                        );

                        None
                    }

                    #[allow(clippy::needless_lifetimes)]
                    fn test_try_from_mut<'bytes>(&mut self, _bytes: &'bytes mut [u8]) -> Option<Option<&'bytes mut $ty>> {
                        assert_on_allowlist!(
                            test_try_from_mut($ty):
                            Option<Box<UnsafeCell<NotZerocopy>>>,
                            Option<&'static UnsafeCell<NotZerocopy>>,
                            Option<&'static mut UnsafeCell<NotZerocopy>>,
                            Option<NonNull<UnsafeCell<NotZerocopy>>>,
                            Option<fn()>,
                            Option<FnManyArgs>,
                            Option<extern "C" fn()>,
                            Option<ECFnManyArgs>,
                            *const NotZerocopy,
                            *mut NotZerocopy
                        );

                        None
                    }

                    fn test_try_read_from(&mut self, _bytes: &[u8]) -> Option<Option<&$ty>> {
                        assert_on_allowlist!(
                            test_try_read_from($ty):
                            str,
                            ManuallyDrop<[u8]>,
                            ManuallyDrop<[bool]>,
                            ManuallyDrop<[UnsafeCell<bool>]>,
                            [u8],
                            [bool]
                        );

                        None
                    }

                    fn test_as_bytes(&mut self, _t: &ReadOnly<$ty>) -> Option<&[u8]> {
                        assert_on_allowlist!(
                            test_as_bytes($ty):
                            Option<&'static UnsafeCell<NotZerocopy>>,
                            Option<&'static mut UnsafeCell<NotZerocopy>>,
                            Option<NonNull<UnsafeCell<NotZerocopy>>>,
                            Option<Box<UnsafeCell<NotZerocopy>>>,
                            Option<fn()>,
                            Option<FnManyArgs>,
                            Option<extern "C" fn()>,
                            Option<ECFnManyArgs>,
                            CoreMaybeUninit<u8>,
                            CoreMaybeUninit<NotZerocopy>,
                            CoreMaybeUninit<UnsafeCell<()>>,
                            ManuallyDrop<UnsafeCell<()>>,
                            ManuallyDrop<[UnsafeCell<u8>]>,
                            ManuallyDrop<[UnsafeCell<bool>]>,
                            Wrapping<UnsafeCell<()>>,
                            *const NotZerocopy,
                            *mut NotZerocopy
                        );

                        None
                    }
                }

                <$ty as TryFromBytesTestable>::with_passing_test_cases(|mut val| {
                    // FIXME(#494): These tests only get exercised for types
                    // which are `IntoBytes`. Once we implement #494, we should
                    // be able to support non-`IntoBytes` types by zeroing
                    // padding.

                    // We define `w` and `ww` since, in the case of the inherent
                    // methods, Rust thinks they're both borrowed mutably at the
                    // same time (given how we use them below). If we just
                    // defined a single `w` and used it for multiple operations,
                    // this would conflict.
                    //
                    // We `#[allow(unused_mut]` for the cases where the "real"
                    // impls are used, which take `&self`.
                    #[allow(unused_mut)]
                    let (mut w, mut ww) = (AutorefWrapper::<$ty>(PhantomData), AutorefWrapper::<$ty>(PhantomData));

                    let c = Ptr::from_ref(&*val);
                    let c = c.forget_aligned();
                    // SAFETY: FIXME(#899): This is unsound. `$ty` is not
                    // necessarily `IntoBytes`, but that's the corner we've
                    // backed ourselves into by using `Ptr::from_ref`.
                    let c = unsafe { c.assume_initialized() };
                    let res = w.test_is_safe_shared(c);
                    if let Some(res) = res {
                        assert!(res, "{}::is_safe (shared `Ptr`): got false, expected true", stringify!($ty));
                    }

                    let c = Ptr::from_mut(&mut *val);
                    let c = c.forget_aligned();
                    // SAFETY: FIXME(#899): This is unsound. `$ty` is not
                    // necessarily `IntoBytes`, but that's the corner we've
                    // backed ourselves into by using `Ptr::from_ref`.
                    let mut c = unsafe { c.assume_initialized() };
                    let res = <$ty as TryFromBytes>::is_safe(c.reborrow_shared());
                    assert!(res, "{}::is_safe (exclusive `Ptr`): got false, expected true", stringify!($ty));

                    // `bytes` is `Some(val.as_bytes())` if `$ty: IntoBytes +
                    // Immutable` and `None` otherwise.
                    let bytes = w.test_as_bytes(&*val);

                    // The inner closure returns
                    // `Some($ty::try_ref_from_bytes(bytes))` if `$ty:
                    // Immutable` and `None` otherwise.
                    let res = bytes.and_then(|bytes| ww.test_try_from_ref(bytes));
                    if let Some(res) = res {
                        assert!(res.is_some(), "{}::try_ref_from_bytes: got `None`, expected `Some`", stringify!($ty));
                    }

                    if let Some(bytes) = bytes {
                        // We need to get a mutable byte slice, and so we clone
                        // into a `Vec`. However, we also need these bytes to
                        // satisfy `$ty`'s alignment requirement, which isn't
                        // guaranteed for `Vec<u8>`. In order to get around
                        // this, we create a `Vec` which is twice as long as we
                        // need. There is guaranteed to be an aligned byte range
                        // of size `size_of_val(val)` within that range.
                        let val = &*val;
                        let size = mem::size_of_val(val);
                        let align = mem::align_of_val(val);

                        let mut vec = bytes.to_vec();
                        vec.extend(bytes);
                        let slc = vec.as_slice();
                        let offset = slc.as_ptr().align_offset(align);
                        let bytes_mut = &mut vec.as_mut_slice()[offset..offset+size];
                        bytes_mut.copy_from_slice(bytes);

                        let res = ww.test_try_from_mut(bytes_mut);
                        if let Some(res) = res {
                            assert!(res.is_some(), "{}::try_mut_from_bytes: got `None`, expected `Some`", stringify!($ty));
                        }
                    }

                    let res = bytes.and_then(|bytes| ww.test_try_read_from(bytes));
                    if let Some(res) = res {
                        assert!(res.is_some(), "{}::try_read_from_bytes: got `None`, expected `Some`", stringify!($ty));
                    }
                });
                #[allow(clippy::as_conversions)]
                <$ty as TryFromBytesTestable>::with_failing_test_cases(|c| {
                    #[allow(unused_mut)] // For cases where the "real" impls are used, which take `&self`.
                    let mut w = AutorefWrapper::<$ty>(PhantomData);

                    // This is `Some($ty::try_ref_from_bytes(c))` if `$ty:
                    // Immutable` and `None` otherwise.
                    let res = w.test_try_from_ref(c);
                    if let Some(res) = res {
                        assert!(res.is_none(), "{}::try_ref_from_bytes({:?}): got Some, expected None", stringify!($ty), c);
                    }

                    let res = w.test_try_from_mut(c);
                    if let Some(res) = res {
                        assert!(res.is_none(), "{}::try_mut_from_bytes({:?}): got Some, expected None", stringify!($ty), c);
                    }


                    let res = w.test_try_read_from(c);
                    if let Some(res) = res {
                        assert!(res.is_none(), "{}::try_read_from_bytes({:?}): got Some, expected None", stringify!($ty), c);
                    }
                });

                #[allow(dead_code)]
                const _: () = { static_assertions::assert_impl_all!($ty: TryFromBytes); };
            };
            ($ty:ty: $trait:ident) => {
                #[allow(dead_code)]
                const _: () = { static_assertions::assert_impl_all!($ty: $trait); };
            };
            ($ty:ty: !$trait:ident) => {
                #[allow(dead_code)]
                const _: () = { static_assertions::assert_not_impl_any!($ty: $trait); };
            };
            ($ty:ty: $($trait:ident),* $(,)? $(!$negative_trait:ident),*) => {
                $(
                    assert_impls!($ty: $trait);
                )*

                $(
                    assert_impls!($ty: !$negative_trait);
                )*
            };
        }

        // NOTE: The negative impl assertions here are not necessarily
        // prescriptive. They merely serve as change detectors to make sure
        // we're aware of what trait impls are getting added with a given
        // change. Of course, some impls would be invalid (e.g., `bool:
        // FromBytes`), and so this change detection is very important.

        assert_impls!(
            (): KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            Unaligned
        );
        assert_impls!(
            u8: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            Unaligned
        );
        assert_impls!(
            i8: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            Unaligned
        );
        assert_impls!(
            u16: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            i16: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            u32: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            i32: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            u64: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            i64: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            u128: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            i128: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            usize: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            isize: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        #[cfg(feature = "float-nightly")]
        assert_impls!(
            f16: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            f32: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            f64: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        #[cfg(feature = "float-nightly")]
        assert_impls!(
            f128: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            !Unaligned
        );
        assert_impls!(
            bool: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            IntoBytes,
            Unaligned,
            !FromBytes
        );
        assert_impls!(
            char: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            str: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            IntoBytes,
            Unaligned,
            !FromBytes
        );

        assert_impls!(
            NonZeroU8: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            Unaligned,
            !FromZeros,
            !FromBytes
        );
        assert_impls!(
            NonZeroI8: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            Unaligned,
            !FromZeros,
            !FromBytes
        );
        assert_impls!(
            NonZeroU16: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroI16: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroU32: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroI32: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroU64: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroI64: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroU128: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroI128: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroUsize: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );
        assert_impls!(
            NonZeroIsize: KnownLayout,
            Immutable,
            TryFromBytes,
            IntoBytes,
            !FromBytes,
            !Unaligned
        );

        assert_impls!(Option<NonZeroU8>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        assert_impls!(Option<NonZeroI8>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        assert_impls!(Option<NonZeroU16>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroI16>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroU32>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroI32>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroU64>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroI64>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroU128>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroI128>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroUsize>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);
        assert_impls!(Option<NonZeroIsize>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned);

        // Implements none of the ZC traits.
        struct NotZerocopy;

        #[rustfmt::skip]
        type FnManyArgs = fn(
            NotZerocopy, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8,
        ) -> (NotZerocopy, NotZerocopy);

        // Allowed, because we're not actually using this type for FFI.
        #[allow(improper_ctypes_definitions)]
        #[rustfmt::skip]
        type ECFnManyArgs = extern "C" fn(
            NotZerocopy, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8,
        ) -> (NotZerocopy, NotZerocopy);

        #[cfg(feature = "alloc")]
        assert_impls!(Option<Box<UnsafeCell<NotZerocopy>>>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<Box<[UnsafeCell<NotZerocopy>]>>: KnownLayout, !Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<&'static UnsafeCell<NotZerocopy>>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<&'static [UnsafeCell<NotZerocopy>]>: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<&'static mut UnsafeCell<NotZerocopy>>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<&'static mut [UnsafeCell<NotZerocopy>]>: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<NonNull<UnsafeCell<NotZerocopy>>>: KnownLayout, TryFromBytes, FromZeros, Immutable, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<NonNull<[UnsafeCell<NotZerocopy>]>>: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<fn()>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<FnManyArgs>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<extern "C" fn()>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Option<ECFnManyArgs>: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);

        assert_impls!(PhantomData<NotZerocopy>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        assert_impls!(PhantomData<UnsafeCell<()>>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        assert_impls!(PhantomData<[u8]>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);

        assert_impls!(ManuallyDrop<u8>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        // This test is important because it allows us to test our hand-rolled
        // implementation of `<ManuallyDrop<T> as TryFromBytes>::is_safe`.
        assert_impls!(ManuallyDrop<bool>: KnownLayout, Immutable, TryFromBytes, FromZeros, IntoBytes, Unaligned, !FromBytes);
        assert_impls!(ManuallyDrop<[u8]>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        // This test is important because it allows us to test our hand-rolled
        // implementation of `<ManuallyDrop<T> as TryFromBytes>::is_safe`.
        assert_impls!(ManuallyDrop<[bool]>: KnownLayout, Immutable, TryFromBytes, FromZeros, IntoBytes, Unaligned, !FromBytes);
        assert_impls!(ManuallyDrop<NotZerocopy>: !Immutable, !TryFromBytes, !KnownLayout, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(ManuallyDrop<[NotZerocopy]>: KnownLayout, !Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(ManuallyDrop<UnsafeCell<()>>: KnownLayout, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned, !Immutable);
        assert_impls!(ManuallyDrop<[UnsafeCell<u8>]>: KnownLayout, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned, !Immutable);
        assert_impls!(ManuallyDrop<[UnsafeCell<bool>]>: KnownLayout, TryFromBytes, FromZeros, IntoBytes, Unaligned, !Immutable, !FromBytes);

        assert_impls!(CoreMaybeUninit<u8>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, Unaligned, !IntoBytes);
        assert_impls!(CoreMaybeUninit<NotZerocopy>: KnownLayout, TryFromBytes, FromZeros, FromBytes, !Immutable, !IntoBytes, !Unaligned);
        assert_impls!(CoreMaybeUninit<UnsafeCell<()>>: KnownLayout, TryFromBytes, FromZeros, FromBytes, Unaligned, !Immutable, !IntoBytes);

        assert_impls!(Wrapping<u8>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        // This test is important because it allows us to test our hand-rolled
        // implementation of `<Wrapping<T> as TryFromBytes>::is_safe`.
        assert_impls!(Wrapping<bool>: KnownLayout, Immutable, TryFromBytes, FromZeros, IntoBytes, Unaligned, !FromBytes);
        assert_impls!(Wrapping<NotZerocopy>: KnownLayout, !Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(Wrapping<UnsafeCell<()>>: KnownLayout, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned, !Immutable);

        assert_impls!(Unalign<u8>: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, Unaligned);
        // This test is important because it allows us to test our hand-rolled
        // implementation of `<Unalign<T> as TryFromBytes>::is_safe`.
        assert_impls!(Unalign<bool>: KnownLayout, Immutable, TryFromBytes, FromZeros, IntoBytes, Unaligned, !FromBytes);
        assert_impls!(Unalign<NotZerocopy>: KnownLayout, Unaligned, !Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes);

        assert_impls!(
            [u8]: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            Unaligned
        );
        assert_impls!(
            [bool]: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            IntoBytes,
            Unaligned,
            !FromBytes
        );
        assert_impls!([NotZerocopy]: KnownLayout, !Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(
            [u8; 0]: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            Unaligned,
        );
        assert_impls!(
            [NotZerocopy; 0]: KnownLayout,
            !Immutable,
            !TryFromBytes,
            !FromZeros,
            !FromBytes,
            !IntoBytes,
            !Unaligned
        );
        assert_impls!(
            [u8; 1]: KnownLayout,
            Immutable,
            TryFromBytes,
            FromZeros,
            FromBytes,
            IntoBytes,
            Unaligned,
        );
        assert_impls!(
            [NotZerocopy; 1]: KnownLayout,
            !Immutable,
            !TryFromBytes,
            !FromZeros,
            !FromBytes,
            !IntoBytes,
            !Unaligned
        );

        assert_impls!(*const NotZerocopy: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(*mut NotZerocopy: KnownLayout, Immutable, TryFromBytes, FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(*const [NotZerocopy]: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(*mut [NotZerocopy]: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(*const dyn Debug: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);
        assert_impls!(*mut dyn Debug: KnownLayout, Immutable, !TryFromBytes, !FromZeros, !FromBytes, !IntoBytes, !Unaligned);

        #[cfg(feature = "simd")]
        {
            #[allow(unused_macros)]
            macro_rules! test_simd_arch_mod {
                ($arch:ident, $($typ:ident),*) => {
                    {
                        use core::arch::$arch::{$($typ),*};
                        use crate::*;
                        $( assert_impls!($typ: KnownLayout, Immutable, TryFromBytes, FromZeros, FromBytes, IntoBytes, !Unaligned); )*
                    }
                };
            }
            #[cfg(target_arch = "x86")]
            test_simd_arch_mod!(x86, __m128, __m128d, __m128i, __m256, __m256d, __m256i);

            #[cfg(all(not(no_zerocopy_simd_x86_avx12_1_89_0), target_arch = "x86"))]
            test_simd_arch_mod!(x86, __m512bh, __m512, __m512d, __m512i);

            #[cfg(target_arch = "x86_64")]
            test_simd_arch_mod!(x86_64, __m128, __m128d, __m128i, __m256, __m256d, __m256i);

            #[cfg(all(not(no_zerocopy_simd_x86_avx12_1_89_0), target_arch = "x86_64"))]
            test_simd_arch_mod!(x86_64, __m512bh, __m512, __m512d, __m512i);

            #[cfg(target_arch = "wasm32")]
            test_simd_arch_mod!(wasm32, v128);

            #[cfg(all(feature = "simd-nightly", target_arch = "powerpc"))]
            test_simd_arch_mod!(
                powerpc,
                vector_bool_long,
                vector_double,
                vector_signed_long,
                vector_unsigned_long
            );

            #[cfg(all(feature = "simd-nightly", target_arch = "powerpc64"))]
            test_simd_arch_mod!(
                powerpc64,
                vector_bool_long,
                vector_double,
                vector_signed_long,
                vector_unsigned_long
            );
            #[cfg(all(target_arch = "aarch64", not(no_zerocopy_aarch64_simd_1_59_0)))]
            #[rustfmt::skip]
            test_simd_arch_mod!(
                aarch64, float32x2_t, float32x4_t, float64x1_t, float64x2_t, int8x8_t, int8x8x2_t,
                int8x8x3_t, int8x8x4_t, int8x16_t, int8x16x2_t, int8x16x3_t, int8x16x4_t, int16x4_t,
                int16x8_t, int32x2_t, int32x4_t, int64x1_t, int64x2_t, poly8x8_t, poly8x8x2_t, poly8x8x3_t,
                poly8x8x4_t, poly8x16_t, poly8x16x2_t, poly8x16x3_t, poly8x16x4_t, poly16x4_t, poly16x8_t,
                poly64x1_t, poly64x2_t, uint8x8_t, uint8x8x2_t, uint8x8x3_t, uint8x8x4_t, uint8x16_t,
                uint8x16x2_t, uint8x16x3_t, uint8x16x4_t, uint16x4_t, uint16x4x2_t, uint16x4x3_t,
                uint16x4x4_t, uint16x8_t, uint32x2_t, uint32x4_t, uint64x1_t, uint64x2_t
            );
        }
    }
}
