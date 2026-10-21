#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu;
    use crate::testutil as test;

    // Type-level representation of an arbitrary modulus.
    struct M {}

    impl PublicModulus for M {}

    #[test]
    fn test_elem_exp_consttime() {
        let cpu_features = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/crypto/fipsmodule/bn/test/mod_exp_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let m = consume_modulus::<M>(test_case, "M");
                let m = m.modulus(cpu_features);
                let expected_result = consume_elem(test_case, "ModExp", &m);
                let base = consume_elem(test_case, "A", &m);
                let e = {
                    let bytes = test_case.consume_bytes("E");
                    PrivateExponent::from_be_bytes_for_test_only(untrusted::Input::from(&bytes), &m)
                        .expect("valid exponent")
                };

                let oneRR = One::newRR(m.alloc_zero(), &m);
                let oneRRR = One::newRRR(oneRR, &m);

                // `base` in the test vectors is reduced (mod M) already but
                // the API expects the bsae to be (mod N) where N = M * P for
                // some other prime of the same length. Fake that here.
                // Pretend there's another prime of equal length.
                struct N {}
                let other_modulus_len_bits = m.len_bits();
                let base: Elem<N> = {
                    let mut limbs = BoxedLimbs::zero(base.limbs.len() * 2);
                    limbs[..base.limbs.len()].copy_from_slice(&base.limbs);
                    Elem {
                        limbs,
                        encoding: PhantomData,
                    }
                };

                let too_big = m.limbs().len() > ELEM_EXP_CONSTTIME_MAX_MODULUS_LIMBS;
                let actual_result = if !too_big {
                    elem_exp_consttime(
                        m.alloc_zero(),
                        &base,
                        &oneRRR,
                        &e,
                        &m,
                        other_modulus_len_bits,
                    )
                } else {
                    let actual_result = elem_exp_consttime(
                        m.alloc_zero(),
                        &base,
                        &oneRRR,
                        &e,
                        &m,
                        other_modulus_len_bits,
                    );
                    // TODO: Be more specific with which error we expect?
                    assert!(actual_result.is_err());
                    // Try again with a larger-than-normally-supported limit
                    elem_exp_consttime_inner::<_, _, { (4096 / LIMB_BITS) * STORAGE_ENTRIES }>(
                        m.alloc_zero(),
                        &base,
                        &oneRRR,
                        &e,
                        &m,
                        other_modulus_len_bits,
                    )
                };
                match actual_result {
                    Ok(r) => assert_elem_eq(&r, &expected_result),
                    Err(LimbSliceError::LenMismatch { .. }) => panic!(),
                    Err(LimbSliceError::TooLong { .. }) => panic!(),
                    Err(LimbSliceError::TooShort { .. }) => panic!(),
                };

                Ok(())
            },
        )
    }

    // TODO: fn test_elem_exp_vartime() using
    // "src/rsa/bigint_elem_exp_vartime_tests.txt". See that file for details.
    // In the meantime, the function is tested indirectly via the RSA
    // verification and signing tests.
    #[test]
    fn test_elem_mul() {
        let cpu_features = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/crypto/fipsmodule/bn/test/mod_mul_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let m = consume_modulus::<M>(test_case, "M");
                let m = m.modulus(cpu_features);
                let expected_result = consume_elem(test_case, "ModMul", &m);
                let a = consume_elem(test_case, "A", &m);
                let b = consume_elem(test_case, "B", &m);

                let b = into_encoded(m.alloc_zero(), b, &m);
                let a = into_encoded(m.alloc_zero(), a, &m);
                let actual_result = elem_mul(&a, b, &m);
                let actual_result = actual_result.into_unencoded(&m);
                assert_elem_eq(&actual_result, &expected_result);

                Ok(())
            },
        )
    }

    #[test]
    fn test_elem_squared() {
        let cpu_features = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/src/arithmetic/bigint_elem_squared_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                let m = consume_modulus::<M>(test_case, "M");
                let m = m.modulus(cpu_features);
                let expected_result = consume_elem(test_case, "ModSquare", &m);
                let a = consume_elem(test_case, "A", &m);

                let a = into_encoded(m.alloc_zero(), a, &m);
                let actual_result = elem_squared(a, &m);
                let actual_result = actual_result.into_unencoded(&m);
                assert_elem_eq(&actual_result, &expected_result);

                Ok(())
            },
        )
    }

    #[test]
    fn test_elem_reduced() {
        let cpu_features = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/src/arithmetic/bigint_elem_reduced_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                struct M {}

                let m_ = consume_modulus::<M>(test_case, "M");
                let m = m_.modulus(cpu_features);
                let expected_result = consume_elem(test_case, "R", &m);
                let a =
                    consume_elem_unchecked::<M>(test_case, "A", expected_result.limbs.len() * 2);
                let other_modulus_len_bits = m_.len_bits();

                let actual_result = elem_reduced(m.alloc_zero(), &a, &m, other_modulus_len_bits);
                let oneRR = One::newRR(m.alloc_zero(), &m);
                let actual_result = elem_mul(oneRR.as_ref(), actual_result, &m);
                assert_elem_eq(&actual_result, &expected_result);

                Ok(())
            },
        )
    }

    #[test]
    fn test_elem_reduced_once() {
        let cpu_features = cpu::features();
        test::run(
            test_vector_file!("../../../fixtures/src/arithmetic/bigint_elem_reduced_once_tests.txt"),
            |section, test_case| {
                assert_eq!(section, "");

                struct M {}
                struct O {}
                let m = consume_modulus::<M>(test_case, "m");
                let m = m.modulus(cpu_features);
                let a = consume_elem_unchecked::<O>(test_case, "a", m.limbs().len());
                let expected_result = consume_elem::<M>(test_case, "r", &m);
                let other_modulus_len_bits = m.len_bits();

                let actual_result =
                    elem_reduced_once(m.alloc_zero(), &a, &m, other_modulus_len_bits);
                assert_elem_eq(&actual_result, &expected_result);

                Ok(())
            },
        )
    }

    fn consume_elem<M>(
        test_case: &mut test::TestCase,
        name: &str,
        m: &Modulus<M>,
    ) -> Elem<M, Unencoded> {
        let value = test_case.consume_bytes(name);
        Elem::from_be_bytes_padded(untrusted::Input::from(&value), m).unwrap()
    }

    fn consume_elem_unchecked<M>(
        test_case: &mut test::TestCase,
        name: &str,
        num_limbs: usize,
    ) -> Elem<M, Unencoded> {
        let bytes = test_case.consume_bytes(name);
        let mut limbs = BoxedLimbs::zero(num_limbs);
        limb::parse_big_endian_and_pad_consttime(untrusted::Input::from(&bytes), &mut limbs)
            .unwrap();
        Elem {
            limbs,
            encoding: PhantomData,
        }
    }

    fn consume_modulus<M>(test_case: &mut test::TestCase, name: &str) -> OwnedModulus<M> {
        let value = test_case.consume_bytes(name);
        OwnedModulus::from(
            OwnedModulusValue::from_be_bytes(untrusted::Input::from(&value)).unwrap(),
        )
    }

    fn assert_elem_eq<M, E>(a: &Elem<M, E>, b: &Elem<M, E>) {
        if elem_verify_equal_consttime(a, b).is_err() {
            panic!("{:x?} != {:x?}", &*a.limbs, &*b.limbs);
        }
    }

    fn into_encoded<M>(out: Storage<M>, a: Elem<M, Unencoded>, m: &Modulus<M>) -> Elem<M, R> {
        let oneRR = One::newRR(out, m);
        elem_mul(oneRR.as_ref(), a, m)
    }
}
