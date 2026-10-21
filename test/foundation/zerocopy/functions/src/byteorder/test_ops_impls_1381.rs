#[test]
    fn test_ops_impls() {
        // Test implementations of traits in `core::ops`. Some of these are
        // fairly banal, but some are optimized to perform the operation without
        // swapping byte order (namely, bit-wise operations which are identical
        // regardless of byte order). These are important to test, and while
        // we're testing those anyway, it's trivial to test all of the impls.

        fn test<T, FTT, FTN, FNT, FNN, FNNChecked, FATT, FATN, FANT>(
            op_t_t: FTT,
            op_t_n: FTN,
            op_n_t: FNT,
            op_n_n: FNN,
            op_n_n_checked: Option<FNNChecked>,
            op_assign: Option<(FATT, FATN, FANT)>,
        ) where
            T: ByteOrderType,
            FTT: Fn(T, T) -> T,
            FTN: Fn(T, T::Native) -> T,
            FNT: Fn(T::Native, T) -> T,
            FNN: Fn(T::Native, T::Native) -> T::Native,
            FNNChecked: Fn(T::Native, T::Native) -> Option<T::Native>,
            FATT: Fn(&mut T, T),
            FATN: Fn(&mut T, T::Native),
            FANT: Fn(&mut T::Native, T),
        {
            let mut r = SmallRng::seed_from_u64(RNG_SEED);
            for _ in 0..RAND_ITERS {
                let n0 = T::Native::rand(&mut r);
                let n1 = T::Native::rand(&mut r);
                let t0 = T::new(n0);
                let t1 = T::new(n1);

                // If this operation would overflow/underflow, skip it rather
                // than attempt to catch and recover from panics.
                if matches!(&op_n_n_checked, Some(checked) if checked(n0, n1).is_none()) {
                    continue;
                }

                let t_t_res = op_t_t(t0, t1);
                let t_n_res = op_t_n(t0, n1);
                let n_t_res = op_n_t(n0, t1);
                let n_n_res = op_n_n(n0, n1);

                // For `f32` and `f64`, NaN values are not considered equal to
                // themselves. We store `Option<f32>`/`Option<f64>` and store
                // NaN as `None` so they can still be compared.
                let val_or_none = |t: T| (!T::Native::is_nan(t.get())).then(|| t.get());
                let t_t_res = val_or_none(t_t_res);
                let t_n_res = val_or_none(t_n_res);
                let n_t_res = val_or_none(n_t_res);
                let n_n_res = (!T::Native::is_nan(n_n_res)).then(|| n_n_res);
                assert_eq!(t_t_res, n_n_res);
                assert_eq!(t_n_res, n_n_res);
                assert_eq!(n_t_res, n_n_res);

                if let Some((op_assign_t_t, op_assign_t_n, op_assign_n_t)) = &op_assign {
                    let mut t_t_res = t0;
                    op_assign_t_t(&mut t_t_res, t1);
                    let mut t_n_res = t0;
                    op_assign_t_n(&mut t_n_res, n1);
                    let mut n_t_res = n0;
                    op_assign_n_t(&mut n_t_res, t1);

                    // For `f32` and `f64`, NaN values are not considered equal to
                    // themselves. We store `Option<f32>`/`Option<f64>` and store
                    // NaN as `None` so they can still be compared.
                    let t_t_res = val_or_none(t_t_res);
                    let t_n_res = val_or_none(t_n_res);
                    let n_t_res = (!T::Native::is_nan(n_t_res)).then(|| n_t_res);
                    assert_eq!(t_t_res, n_n_res);
                    assert_eq!(t_n_res, n_n_res);
                    assert_eq!(n_t_res, n_n_res);
                }
            }
        }

        macro_rules! test {
            (
                @binary
                $trait:ident,
                $method:ident $([$checked_method:ident])?,
                $trait_assign:ident,
                $method_assign:ident,
                $($call_for_macros:ident),*
            ) => {{
                fn t<T>()
                where
                    T: ByteOrderType,
                    T: core::ops::$trait<T, Output = T>,
                    T: core::ops::$trait<T::Native, Output = T>,
                    T::Native: core::ops::$trait<T, Output = T>,
                    T::Native: core::ops::$trait<T::Native, Output = T::Native>,

                    T: core::ops::$trait_assign<T>,
                    T: core::ops::$trait_assign<T::Native>,
                    T::Native: core::ops::$trait_assign<T>,
                    T::Native: core::ops::$trait_assign<T::Native>,
                {
                    test::<T, _, _, _, _, _, _, _, _>(
                        core::ops::$trait::$method,
                        core::ops::$trait::$method,
                        core::ops::$trait::$method,
                        core::ops::$trait::$method,
                        {
                            #[allow(unused_mut, unused_assignments)]
                            let mut op_native_checked = None::<fn(T::Native, T::Native) -> Option<T::Native>>;
                            $(
                                op_native_checked = Some(T::Native::$checked_method);
                            )?
                            op_native_checked
                        },
                        Some((
                            <T as core::ops::$trait_assign<T>>::$method_assign,
                            <T as core::ops::$trait_assign::<T::Native>>::$method_assign,
                            <T::Native as core::ops::$trait_assign::<T>>::$method_assign
                        )),
                    );
                }

                $(
                    $call_for_macros!(t, NativeEndian);
                    $call_for_macros!(t, NonNativeEndian);
                )*
            }};
            (
                @unary
                $trait:ident,
                $method:ident,
                $($call_for_macros:ident),*
            ) => {{
                fn t<T>()
                where
                    T: ByteOrderType,
                    T: core::ops::$trait<Output = T>,
                    T::Native: core::ops::$trait<Output = T::Native>,
                {
                    test::<T, _, _, _, _, _, _, _, _>(
                        |slf, _rhs| core::ops::$trait::$method(slf),
                        |slf, _rhs| core::ops::$trait::$method(slf),
                        |slf, _rhs| core::ops::$trait::$method(slf).into(),
                        |slf, _rhs| core::ops::$trait::$method(slf),
                        None::<fn(T::Native, T::Native) -> Option<T::Native>>,
                        None::<(fn(&mut T, T), fn(&mut T, T::Native), fn(&mut T::Native, T))>,
                    );
                }

                $(
                    $call_for_macros!(t, NativeEndian);
                    $call_for_macros!(t, NonNativeEndian);
                )*
            }};
        }

        test!(@binary Add, add[checked_add], AddAssign, add_assign, call_for_all_types);
        test!(@binary Div, div[checked_div], DivAssign, div_assign, call_for_all_types);
        test!(@binary Mul, mul[checked_mul], MulAssign, mul_assign, call_for_all_types);
        test!(@binary Rem, rem[checked_rem], RemAssign, rem_assign, call_for_all_types);
        test!(@binary Sub, sub[checked_sub], SubAssign, sub_assign, call_for_all_types);

        test!(@binary BitAnd, bitand, BitAndAssign, bitand_assign, call_for_unsigned_types, call_for_signed_types);
        test!(@binary BitOr, bitor, BitOrAssign, bitor_assign, call_for_unsigned_types, call_for_signed_types);
        test!(@binary BitXor, bitxor, BitXorAssign, bitxor_assign, call_for_unsigned_types, call_for_signed_types);
        test!(@binary Shl, shl[checked_shl], ShlAssign, shl_assign, call_for_unsigned_types, call_for_signed_types);
        test!(@binary Shr, shr[checked_shr], ShrAssign, shr_assign, call_for_unsigned_types, call_for_signed_types);

        test!(@unary Not, not, call_for_signed_types, call_for_unsigned_types);
        test!(@unary Neg, neg, call_for_signed_types, call_for_float_types);

        for shift in [0u64, 1, 31, 63] {
            let n = 0x0123_4567_89AB_CDEFu64;
            let shift_u32: u32 = shift.try_into().unwrap();
            let shl = n.checked_shl(shift_u32).unwrap();
            let shr = n.checked_shr(shift_u32).unwrap();
            assert_eq!(core::ops::Shl::shl(U64::<NativeEndian>::new(n), shift).get(), shl);
            assert_eq!(core::ops::Shr::shr(U64::<NativeEndian>::new(n), shift).get(), shr);
            assert_eq!(core::ops::Shl::shl(U64::<NonNativeEndian>::new(n), shift).get(), shl);
            assert_eq!(core::ops::Shr::shr(U64::<NonNativeEndian>::new(n), shift).get(), shr);
        }
    }
