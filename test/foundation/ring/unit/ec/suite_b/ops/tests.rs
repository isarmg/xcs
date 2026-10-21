#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use crate::testutil as test;
    use alloc::{format, vec, vec::Vec};

    const ZERO_SCALAR: Scalar = Scalar {
        limbs: [0; elem::NumLimbs::MAX],
        m: PhantomData,
        encoding: PhantomData,
    };

    trait Convert<E: Encoding> {
        fn convert(self, q: &Modulus<Q>) -> Elem<E>;
    }

    impl Convert<R> for Elem<R> {
        fn convert(self, _q: &Modulus<Q>) -> Elem<R> {
            self
        }
    }

    impl Convert<Unencoded> for Elem<R> {
        fn convert(self, q: &Modulus<Q>) -> Elem<Unencoded> {
            q.elem_unencoded(&self)
        }
    }

    fn q_minus_n_plus_n_equals_0_test(ops: &PublicScalarOps) {
        let cops = ops.scalar_ops.common;
        let q = &cops.elem_modulus(cpu::features());
        let mut x = Elem::from(&ops.q_minus_n);
        q.add_assign(&mut x, &Elem::from(&cops.n));
        assert!(q.is_zero(&x));
    }

    #[test]
    fn p256_q_minus_n_plus_n_equals_0_test() {
        q_minus_n_plus_n_equals_0_test(&p256::PUBLIC_SCALAR_OPS);
    }

    #[test]
    fn p384_q_minus_n_plus_n_equals_0_test() {
        q_minus_n_plus_n_equals_0_test(&p384::PUBLIC_SCALAR_OPS);
    }

    #[test]
    fn p256_elem_add_test() {
        elem_add_test(
            &p256::PUBLIC_SCALAR_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_elem_sum_tests.txt"),
        );
    }

    #[test]
    fn p384_elem_add_test() {
        elem_add_test(
            &p384::PUBLIC_SCALAR_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_elem_sum_tests.txt"),
        );
    }

    fn elem_add_test(ops: &PublicScalarOps, test_file: test::File) {
        let cops = ops.public_key_ops.common;
        let q = &cops.elem_modulus(cpu::features());
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_elem(q, test_case, "a");
            let b = consume_elem(q, test_case, "b");
            let expected_sum = consume_elem(q, test_case, "r");

            let mut actual_sum = a;
            q.add_assign(&mut actual_sum, &b);
            assert_limbs_are_equal(cops, &actual_sum.limbs, &expected_sum.limbs);

            let mut actual_sum = b;
            q.add_assign(&mut actual_sum, &a);
            assert_limbs_are_equal(cops, &actual_sum.limbs, &expected_sum.limbs);

            Ok(())
        })
    }

    // XXX: There's no `p256_sub` in *ring*; it's logic is inlined into
    // the point arithmetic functions. Thus, we can't test it.

    #[test]
    fn p384_elem_sub_test() {
        prefixed_extern! {
            fn p384_elem_sub(r: *mut Limb, a: *const Limb, b: *const Limb);
        }
        elem_sub_test(
            &p384::COMMON_OPS,
            p384_elem_sub,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_elem_sum_tests.txt"),
        );
    }

    fn elem_sub_test(
        ops: &'static CommonOps,
        elem_sub: unsafe extern "C" fn(r: *mut Limb, a: *const Limb, b: *const Limb),
        test_file: test::File,
    ) {
        let q = &ops.elem_modulus(cpu::features());
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_elem(q, test_case, "a");
            let b = consume_elem(q, test_case, "b");
            let r = consume_elem(q, test_case, "r");

            let mut actual_difference = Elem::<R>::zero();
            unsafe {
                elem_sub(
                    actual_difference.limbs.as_mut_ptr(),
                    r.limbs.as_ptr(),
                    b.limbs.as_ptr(),
                );
            }
            assert_limbs_are_equal(ops, &actual_difference.limbs, &a.limbs);

            let mut actual_difference = Elem::<R>::zero();
            unsafe {
                elem_sub(
                    actual_difference.limbs.as_mut_ptr(),
                    r.limbs.as_ptr(),
                    a.limbs.as_ptr(),
                );
            }
            assert_limbs_are_equal(ops, &actual_difference.limbs, &b.limbs);

            Ok(())
        })
    }

    // XXX: There's no `p256_div_by_2` in *ring*; it's logic is inlined
    // into the point arithmetic functions. Thus, we can't test it.

    #[test]
    fn p384_elem_div_by_2_test() {
        prefixed_extern! {
            fn p384_elem_div_by_2(r: *mut Limb, a: *const Limb);
        }
        elem_div_by_2_test(
            &p384::COMMON_OPS,
            p384_elem_div_by_2,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_elem_div_by_2_tests.txt"),
        );
    }

    fn elem_div_by_2_test(
        ops: &'static CommonOps,
        elem_div_by_2: unsafe extern "C" fn(r: *mut Limb, a: *const Limb),
        test_file: test::File,
    ) {
        let q = &ops.elem_modulus(cpu::features());
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_elem(q, test_case, "a");
            let r = consume_elem(q, test_case, "r");

            let mut actual_result = Elem::<R>::zero();
            unsafe {
                elem_div_by_2(actual_result.limbs.as_mut_ptr(), a.limbs.as_ptr());
            }
            assert_limbs_are_equal(ops, &actual_result.limbs, &r.limbs);

            Ok(())
        })
    }

    // There is no `ecp_nistz256_neg` on other targets.
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn p256_elem_neg_test() {
        prefixed_extern! {
            fn ecp_nistz256_neg(r: *mut Limb, a: *const Limb);
        }
        elem_neg_test(
            &p256::COMMON_OPS,
            ecp_nistz256_neg,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_elem_neg_tests.txt"),
        );
    }

    #[test]
    fn p384_elem_neg_test() {
        prefixed_extern! {
            fn p384_elem_neg(r: *mut Limb, a: *const Limb);
        }
        elem_neg_test(
            &p384::COMMON_OPS,
            p384_elem_neg,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_elem_neg_tests.txt"),
        );
    }

    fn elem_neg_test(
        ops: &'static CommonOps,
        elem_neg: unsafe extern "C" fn(r: *mut Limb, a: *const Limb),
        test_file: test::File,
    ) {
        let q = &ops.elem_modulus(cpu::features());
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_elem(q, test_case, "a");
            let b = consume_elem(q, test_case, "b");

            // Verify -a == b.
            {
                let mut actual_result = Elem::<R>::zero();
                unsafe {
                    elem_neg(actual_result.limbs.as_mut_ptr(), a.limbs.as_ptr());
                }
                assert_limbs_are_equal(ops, &actual_result.limbs, &b.limbs);
            }

            // Verify -b == a.
            {
                let mut actual_result = Elem::<R>::zero();
                unsafe {
                    elem_neg(actual_result.limbs.as_mut_ptr(), b.limbs.as_ptr());
                }
                assert_limbs_are_equal(ops, &actual_result.limbs, &a.limbs);
            }

            Ok(())
        })
    }

    #[test]
    fn p256_elem_mul_test() {
        elem_mul_test(
            &p256::COMMON_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_elem_mul_tests.txt"),
        );
    }

    #[test]
    fn p384_elem_mul_test() {
        elem_mul_test(
            &p384::COMMON_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_elem_mul_tests.txt"),
        );
    }

    fn elem_mul_test(ops: &'static CommonOps, test_file: test::File) {
        let q = &ops.elem_modulus(cpu::features());
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let mut a = consume_elem(q, test_case, "a");
            let b = consume_elem(q, test_case, "b");
            let r = consume_elem(q, test_case, "r");
            q.elem_mul(&mut a, &b);
            assert_limbs_are_equal(ops, &a.limbs, &r.limbs);

            Ok(())
        })
    }

    #[test]
    fn p256_scalar_mul_test() {
        scalar_mul_test(
            &p256::SCALAR_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_scalar_mul_tests.txt"),
        );
    }

    #[test]
    fn p384_scalar_mul_test() {
        scalar_mul_test(
            &p384::SCALAR_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_scalar_mul_tests.txt"),
        );
    }

    fn scalar_mul_test(ops: &ScalarOps, test_file: test::File) {
        let cpu = cpu::features();
        let cops = ops.common;
        let n = &cops.scalar_modulus(cpu);
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");
            let a = consume_scalar(n, test_case, "a");
            let b = consume_scalar_mont(n, test_case, "b");
            let expected_result = consume_scalar(n, test_case, "r");
            let actual_result = ops.scalar_product(&a, &b, cpu);
            assert_limbs_are_equal(cops, &actual_result.limbs, &expected_result.limbs);

            Ok(())
        })
    }

    #[test]
    fn p256_scalar_square_test() {
        prefixed_extern! {
            fn p256_scalar_sqr_rep_mont(r: *mut Limb, a: *const Limb, rep: LeakyWord);
        }
        scalar_square_test(
            &p256::SCALAR_OPS,
            p256_scalar_sqr_rep_mont,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_scalar_square_tests.txt"),
        );
    }

    // XXX: There's no `p384_scalar_square_test()` because there's no dedicated
    // `p384_scalar_sqr_rep_mont()`.

    fn scalar_square_test(
        ops: &ScalarOps,
        sqr_rep: unsafe extern "C" fn(r: *mut Limb, a: *const Limb, rep: LeakyWord),
        test_file: test::File,
    ) {
        let cpu = cpu::features();
        let cops = ops.common;
        let n = &cops.scalar_modulus(cpu);
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");
            let cpu = cpu::features();
            let a = consume_scalar(n, test_case, "a");
            let expected_result = consume_scalar(n, test_case, "r");

            {
                let mut actual_result: Scalar<R> = Scalar {
                    limbs: [0; elem::NumLimbs::MAX],
                    m: PhantomData,
                    encoding: PhantomData,
                };
                unsafe {
                    sqr_rep(actual_result.limbs.as_mut_ptr(), a.limbs.as_ptr(), 1);
                }
                assert_limbs_are_equal(cops, &actual_result.limbs, &expected_result.limbs);
            }

            {
                let actual_result = ops.scalar_product(&a, &a, cpu);
                assert_limbs_are_equal(cops, &actual_result.limbs, &expected_result.limbs);
            }

            Ok(())
        })
    }

    #[test]
    #[should_panic(expected = "!self.scalar_ops.common.is_zero(a)")]
    fn p256_scalar_inv_to_mont_zero_panic_test() {
        let _ = p256::PRIVATE_SCALAR_OPS.scalar_inv_to_mont(&ZERO_SCALAR, cpu::features());
    }

    #[test]
    #[should_panic(expected = "!self.scalar_ops.common.is_zero(a)")]
    fn p384_scalar_inv_to_mont_zero_panic_test() {
        let _ = p384::PRIVATE_SCALAR_OPS.scalar_inv_to_mont(&ZERO_SCALAR, cpu::features());
    }

    #[test]
    fn p256_point_sum_test() {
        point_sum_test(
            &p256::PRIVATE_KEY_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_point_sum_tests.txt"),
        );
    }

    #[test]
    fn p384_point_sum_test() {
        point_sum_test(
            &p384::PRIVATE_KEY_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_point_sum_tests.txt"),
        );
    }

    fn point_sum_test(ops: &PrivateKeyOps, test_file: test::File) {
        let cpu = cpu::features();

        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_jacobian_point(ops, test_case, "a");
            let b = consume_jacobian_point(ops, test_case, "b");
            let r_expected: TestPoint<R> = consume_point(ops, test_case, "r");

            let r_actual = ops.point_sum(&a, &b, cpu);
            assert_point_actual_equals_expected(ops, &r_actual, &r_expected);

            Ok(())
        });
    }

    #[test]
    fn p256_point_sum_mixed_test() {
        prefixed_extern! {
            fn p256_point_add_affine(
                r: *mut Limb,   // [p256::COMMON_OPS.num_limbs*3]
                a: *const Limb, // [p256::COMMON_OPS.num_limbs*3]
                b: *const Limb, // [p256::COMMON_OPS.num_limbs*2]
            );
        }
        point_sum_mixed_test(
            &p256::PRIVATE_KEY_OPS,
            p256_point_add_affine,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_point_sum_mixed_tests.txt"),
        );
    }

    // XXX: There is no `nistz384_point_add_affine()`.

    fn point_sum_mixed_test(
        ops: &PrivateKeyOps,
        point_add_affine: unsafe extern "C" fn(
            r: *mut Limb,   // [ops.num_limbs*3]
            a: *const Limb, // [ops.num_limbs*3]
            b: *const Limb, // [ops.num_limbs*2]
        ),
        test_file: test::File,
    ) {
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_jacobian_point(ops, test_case, "a");
            let b = consume_affine_point(ops, test_case, "b");
            let r_expected: TestPoint<R> = consume_point(ops, test_case, "r");

            let mut r_actual = Point::new_at_infinity();
            unsafe {
                point_add_affine(r_actual.xyz.as_mut_ptr(), a.xyz.as_ptr(), b.xy.as_ptr());
            }

            assert_point_actual_equals_expected(ops, &r_actual, &r_expected);

            Ok(())
        });
    }

    #[test]
    fn p256_point_double_test() {
        prefixed_extern! {
            fn p256_point_double(
                r: *mut Limb,   // [p256::COMMON_OPS.num_limbs*3]
                a: *const Limb, // [p256::COMMON_OPS.num_limbs*3]
            );
        }
        point_double_test(
            &p256::PRIVATE_KEY_OPS,
            p256_point_double,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_point_double_tests.txt"),
        );
    }

    #[test]
    fn p384_point_double_test() {
        prefixed_extern! {
            fn p384_point_double(
                r: *mut Limb,   // [p384::COMMON_OPS.num_limbs*3]
                a: *const Limb, // [p384::COMMON_OPS.num_limbs*3]
            );
        }
        point_double_test(
            &p384::PRIVATE_KEY_OPS,
            p384_point_double,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_point_double_tests.txt"),
        );
    }

    fn point_double_test(
        ops: &PrivateKeyOps,
        point_double: unsafe extern "C" fn(
            r: *mut Limb,   // [ops.num_limbs*3]
            a: *const Limb, // [ops.num_limbs*3]
        ),
        test_file: test::File,
    ) {
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");

            let a = consume_jacobian_point(ops, test_case, "a");
            let r_expected: TestPoint<R> = consume_point(ops, test_case, "r");

            let mut r_actual = Point::new_at_infinity();
            unsafe {
                point_double(r_actual.xyz.as_mut_ptr(), a.xyz.as_ptr());
            }

            assert_point_actual_equals_expected(ops, &r_actual, &r_expected);

            Ok(())
        });
    }

    /// TODO: We should be testing `point_mul` with points other than the generator.
    #[test]
    fn p256_point_mul_test() {
        let generator = (
            Elem::from(&p256::GENERATOR.0),
            Elem::from(&p256::GENERATOR.1),
        );
        point_mul_base_tests(
            &p256::PRIVATE_KEY_OPS,
            |s, cpu| p256::PRIVATE_KEY_OPS.point_mul(s, &generator, cpu),
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_point_mul_base_tests.txt"),
        );
    }

    /// TODO: We should be testing `point_mul` with points other than the generator.
    #[test]
    fn p384_point_mul_test() {
        let generator = (
            Elem::from(&p384::GENERATOR.0),
            Elem::from(&p384::GENERATOR.1),
        );

        point_mul_base_tests(
            &p384::PRIVATE_KEY_OPS,
            |s, cpu| p384::PRIVATE_KEY_OPS.point_mul(s, &generator, cpu),
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_point_mul_base_tests.txt"),
        );
    }

    #[test]
    fn p256_point_mul_serialized_test() {
        point_mul_serialized_test(
            &p256::PRIVATE_KEY_OPS,
            &p256::PUBLIC_KEY_OPS,
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_point_mul_serialized_tests.txt"),
        );
    }

    fn point_mul_serialized_test(
        priv_ops: &PrivateKeyOps,
        pub_ops: &PublicKeyOps,
        test_file: test::File,
    ) {
        let cpu = cpu::features();
        let cops = pub_ops.common;
        let q = &cops.elem_modulus(cpu);
        let n = &cops.scalar_modulus(cpu);
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");
            let p_scalar = consume_scalar(n, test_case, "p_scalar");

            let p = test_case.consume_bytes("p");
            let p = super::super::public_key::parse_uncompressed_point(
                pub_ops,
                q,
                untrusted::Input::from(&p),
            )
            .expect("valid point");

            let expected_result = test_case.consume_bytes("r");

            let product = priv_ops.point_mul(&p_scalar, &p, cpu::features());

            let mut actual_result = vec![4u8; 1 + (2 * cops.len())];
            {
                let (x, y) = actual_result[1..].split_at_mut(cops.len());
                super::super::private_key::big_endian_affine_from_jacobian(
                    priv_ops,
                    q,
                    x,
                    Some(y),
                    &product,
                )
                .expect("successful encoding");
            }

            assert_eq!(expected_result, actual_result);

            Ok(())
        })
    }

    #[test]
    fn p256_point_mul_base_test() {
        point_mul_base_tests(
            &p256::PRIVATE_KEY_OPS,
            |s, cpu| p256::PRIVATE_KEY_OPS.point_mul_base(s, cpu),
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p256_point_mul_base_tests.txt"),
        );
    }

    #[test]
    fn p384_point_mul_base_test() {
        point_mul_base_tests(
            &p384::PRIVATE_KEY_OPS,
            |s, cpu| p384::PRIVATE_KEY_OPS.point_mul_base(s, cpu),
            test_vector_file!("../../../../fixtures/src/ec/suite_b/ops/p384_point_mul_base_tests.txt"),
        );
    }

    pub(super) fn point_mul_base_tests(
        ops: &PrivateKeyOps,
        f: impl Fn(&Scalar, cpu::Features) -> Point,
        test_file: test::File,
    ) {
        let cpu = cpu::features();
        let n = &ops.common.scalar_modulus(cpu);
        test::run(test_file, |section, test_case| {
            assert_eq!(section, "");
            let g_scalar = consume_scalar(n, test_case, "g_scalar");
            let expected_result: TestPoint<Unencoded> = consume_point(ops, test_case, "r");
            let actual_result = f(&g_scalar, cpu);
            assert_point_actual_equals_expected(ops, &actual_result, &expected_result);
            Ok(())
        })
    }

    fn assert_point_actual_equals_expected<E: Encoding>(
        ops: &PrivateKeyOps,
        actual_point: &Point,
        expected_point: &TestPoint<E>,
    ) where
        Elem<R>: Convert<E>,
    {
        let cpu = cpu::features();

        let cops = ops.common;
        let q = &cops.elem_modulus(cpu);
        let actual_x = &q.point_x(actual_point);
        let actual_y = &q.point_y(actual_point);
        let actual_z = &q.point_z(actual_point);
        match expected_point {
            TestPoint::Infinity => {
                let zero = Elem::zero();
                assert_elems_are_equal(q, actual_z, &zero);
            }
            TestPoint::Affine(expected_x, expected_y) => {
                let zz_inv = ops.elem_inverse_squared(q, actual_z);
                let x_aff = q.elem_product(actual_x, &zz_inv);
                let y_aff = {
                    let zzzz_inv = q.elem_squared(&zz_inv);
                    let zzz_inv = q.elem_product(actual_z, &zzzz_inv);
                    q.elem_product(actual_y, &zzz_inv)
                };

                let x_aff = x_aff.convert(q);
                let y_aff = y_aff.convert(q);

                assert_elems_are_equal(q, &x_aff, expected_x);
                assert_elems_are_equal(q, &y_aff, expected_y);
            }
        }
    }

    fn consume_jacobian_point(
        ops: &PrivateKeyOps,
        test_case: &mut test::TestCase,
        name: &str,
    ) -> Point {
        let q = &ops.common.elem_modulus(cpu::features());
        let input = test_case.consume_string(name);
        let elems = input.split(", ").collect::<Vec<&str>>();
        assert_eq!(elems.len(), 3);
        let mut p = Point::new_at_infinity();
        consume_point_elem(q, &mut p.xyz, &elems, 0);
        consume_point_elem(q, &mut p.xyz, &elems, 1);
        consume_point_elem(q, &mut p.xyz, &elems, 2);
        p
    }

    struct AffinePoint {
        xy: [Limb; 2 * elem::NumLimbs::MAX],
    }

    fn consume_affine_point(
        ops: &PrivateKeyOps,
        test_case: &mut test::TestCase,
        name: &str,
    ) -> AffinePoint {
        let q = &ops.common.elem_modulus(cpu::features());
        let input = test_case.consume_string(name);
        let elems = input.split(", ").collect::<Vec<&str>>();
        assert_eq!(elems.len(), 2);
        let mut p = AffinePoint {
            xy: [0; 2 * elem::NumLimbs::MAX],
        };
        consume_point_elem(q, &mut p.xy, &elems, 0);
        consume_point_elem(q, &mut p.xy, &elems, 1);
        p
    }

    fn consume_point_elem(q: &Modulus<Q>, limbs_out: &mut [Limb], elems: &[&str], i: usize) {
        let num_limbs = q.num_limbs.into();
        let bytes = test::from_hex(elems[i]).unwrap();
        let bytes = untrusted::Input::from(&bytes);
        let r: Elem<Unencoded> = elem_parse_big_endian_fixed_consttime(q, bytes).unwrap();
        // XXX: “Transmute” this to `Elem<R>` limbs.
        limbs_out[(i * num_limbs)..((i + 1) * num_limbs)].copy_from_slice(&r.limbs[..num_limbs]);
    }

    enum TestPoint<E: Encoding> {
        Infinity,
        Affine(Elem<E>, Elem<E>),
    }

    fn consume_point<E: Encoding>(
        ops: &PrivateKeyOps,
        test_case: &mut test::TestCase,
        name: &str,
    ) -> TestPoint<E> {
        let q = &ops.common.elem_modulus(cpu::features());
        fn consume_point_elem<E: Encoding>(q: &Modulus<Q>, elems: &[&str], i: usize) -> Elem<E> {
            let bytes = test::from_hex(elems[i]).unwrap();
            let bytes = untrusted::Input::from(&bytes);
            let unencoded: Elem<Unencoded> =
                elem_parse_big_endian_fixed_consttime(q, bytes).unwrap();
            // XXX: “Transmute” this to `Elem<R>` limbs.
            Elem {
                limbs: unencoded.limbs,
                m: PhantomData,
                encoding: PhantomData,
            }
        }

        let input = test_case.consume_string(name);
        if input == "inf" {
            return TestPoint::Infinity;
        }
        let elems = input.split(", ").collect::<Vec<&str>>();
        assert_eq!(elems.len(), 2);
        let x = consume_point_elem(q, &elems, 0);
        let y = consume_point_elem(q, &elems, 1);
        TestPoint::Affine(x, y)
    }

    fn assert_elems_are_equal<E: Encoding>(q: &Modulus<Q>, a: &Elem<E>, b: &Elem<E>) {
        assert_limbs_are_equal(q.cops, &a.limbs, &b.limbs)
    }

    fn assert_limbs_are_equal(
        ops: &CommonOps,
        actual: &[Limb; elem::NumLimbs::MAX],
        expected: &[Limb; elem::NumLimbs::MAX],
    ) {
        let num_limbs = ops.num_limbs.into();
        if actual[..num_limbs] != expected[..num_limbs] {
            let mut actual_s = alloc::string::String::new();
            let mut expected_s = alloc::string::String::new();
            for j in 0..num_limbs {
                let width = LIMB_BITS / 4;
                let formatted = format!("{:0width$x}", actual[num_limbs - j - 1]);
                actual_s.push_str(&formatted);
                let formatted = format!("{:0width$x}", expected[num_limbs - j - 1]);
                expected_s.push_str(&formatted);
            }
            panic!(
                "Actual != Expected,\nActual = {}, Expected = {}",
                actual_s, expected_s
            );
        }
    }

    fn consume_elem(q: &Modulus<Q>, test_case: &mut test::TestCase, name: &str) -> Elem<R> {
        let unpadded_bytes = test_case.consume_bytes(name);
        let mut bytes = vec![0; q.bytes_len() - unpadded_bytes.len()];
        bytes.extend(&unpadded_bytes);

        let bytes = untrusted::Input::from(&bytes);
        let r: Elem<Unencoded> = elem_parse_big_endian_fixed_consttime(q, bytes).unwrap();
        // XXX: “Transmute” this to an `Elem<R>`.
        Elem {
            limbs: r.limbs,
            m: PhantomData,
            encoding: PhantomData,
        }
    }

    fn consume_scalar(n: &Modulus<N>, test_case: &mut test::TestCase, name: &str) -> Scalar {
        let bytes = test_case.consume_bytes(name);
        let bytes = untrusted::Input::from(&bytes);
        scalar_parse_big_endian_variable(n, AllowZero::Yes, bytes).unwrap()
    }

    fn consume_scalar_mont(
        n: &Modulus<N>,
        test_case: &mut test::TestCase,
        name: &str,
    ) -> Scalar<R> {
        let bytes = test_case.consume_bytes(name);
        let bytes = untrusted::Input::from(&bytes);
        let s = scalar_parse_big_endian_variable(n, AllowZero::Yes, bytes).unwrap();
        // “Transmute” it to a `Scalar<R>`.
        Scalar {
            limbs: s.limbs,
            m: PhantomData,
            encoding: PhantomData,
        }
    }
}
