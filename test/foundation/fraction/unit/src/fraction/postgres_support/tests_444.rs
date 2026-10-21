#[cfg(test)]
mod tests {
    use super::*;

    type Fraction = GenericFraction<u128>;
    const NUMERIC_OID: u32 = 1700;

    fn get_tests() -> Vec<(Fraction, &'static [u8])> {
        vec![
            (
                Fraction::new_raw_signed(Sign::Minus, 12345678901234u128, 1u128),
                &[0, 4, 0, 3, 64, 0, 0, 0, 0, 12, 13, 128, 30, 210, 4, 210],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 12345678u128, 1u128),
                &[0, 2, 0, 1, 64, 0, 0, 0, 4, 210, 22, 46],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 10000000000000000001u128, 10000000000u128),
                &[
                    0, 6, 0, 2, 64, 0, 0, 10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100,
                ],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1000000000u128, 1u128),
                &[0, 1, 0, 2, 64, 0, 0, 0, 0, 10],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1234u128, 1u128),
                &[0, 1, 0, 0, 64, 0, 0, 0, 4, 210],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 256u128, 1u128),
                &[0, 1, 0, 0, 64, 0, 0, 0, 1, 0],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 42u128, 1u128),
                &[0, 1, 0, 0, 64, 0, 0, 0, 0, 42],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1u128, 1u128),
                &[0, 1, 0, 0, 64, 0, 0, 0, 0, 1],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1u128, 2u128),
                &[0, 1, 255, 255, 64, 0, 0, 1, 19, 136],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1u128, 10u128),
                &[0, 1, 255, 255, 64, 0, 0, 1, 3, 232],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 66u128, 100u128),
                &[0, 1, 255, 255, 64, 0, 0, 2, 25, 200],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 6172839450617u128, 50000000000000u128),
                &[0, 4, 255, 255, 64, 0, 0, 14, 4, 210, 22, 46, 35, 52, 13, 72],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1u128, 100u128),
                &[0, 1, 255, 255, 64, 0, 0, 2, 0, 100],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 601u128, 2500u128),
                &[0, 1, 255, 255, 64, 0, 0, 4, 9, 100],
            ),
            (
                Fraction::new_raw_signed(Sign::Minus, 1u128, 1000000000u128),
                &[0, 1, 255, 253, 64, 0, 0, 9, 3, 232],
            ),
            (
                Fraction::new_raw_signed(
                    Sign::Minus,
                    617283945061706172839450617u128,
                    50000000000000u128,
                ),
                &[
                    0, 8, 0, 3, 64, 0, 0, 14, 0, 12, 13, 128, 30, 210, 4, 210, 4, 210, 22, 46, 35,
                    52, 13, 72,
                ],
            ),
            (Fraction::zero(), &[0, 0, 0, 0, 0, 0, 0, 0]),
            (Fraction::nan(), &[0, 0, 0, 0, 192, 0, 0, 0]),
            (Fraction::infinity(), &[0, 0, 0, 0, 208, 0, 0, 0]),
            (
                Fraction::new(1u128, 1u128) / Fraction::new(0u128, 1u128),
                &[0, 0, 0, 0, 208, 0, 0, 0],
            ),
            (Fraction::neg_infinity(), &[0, 0, 0, 0, 240, 0, 0, 0]),
            (
                Fraction::new_raw(617283945061706172839450617u128, 50000000000000u128),
                &[
                    0, 8, 0, 3, 0, 0, 0, 14, 0, 12, 13, 128, 30, 210, 4, 210, 4, 210, 22, 46, 35,
                    52, 13, 72,
                ],
            ),
            (
                Fraction::new_raw(1u128, 1000000000u128),
                &[0, 1, 255, 253, 0, 0, 0, 9, 3, 232],
            ),
            (
                Fraction::new_raw(601u128, 2500u128),
                &[0, 1, 255, 255, 0, 0, 0, 4, 9, 100],
            ),
            (
                Fraction::new_raw(1u128, 100u128),
                &[0, 1, 255, 255, 0, 0, 0, 2, 0, 100],
            ),
            (
                Fraction::new_raw(66u128, 100u128),
                &[0, 1, 255, 255, 0, 0, 0, 2, 25, 200],
            ),
            (
                Fraction::new_raw(10000000000000000000001u128, 10000000000000000u128),
                &[
                    0, 6, 0, 1, 0, 0, 0, 16, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ],
            ),
            (
                Fraction::new_raw(6172839450617u128, 50000000000000u128),
                &[0, 4, 255, 255, 0, 0, 0, 14, 4, 210, 22, 46, 35, 52, 13, 72],
            ),
            (
                Fraction::new_raw(1u128, 10u128),
                &[0, 1, 255, 255, 0, 0, 0, 1, 3, 232],
            ),
            (
                Fraction::new_raw(1u128, 2u128),
                &[0, 1, 255, 255, 0, 0, 0, 1, 19, 136],
            ),
            (
                Fraction::new_raw(1u128, 1u128),
                &[0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            ),
            (
                Fraction::new_raw(42u128, 1u128),
                &[0, 1, 0, 0, 0, 0, 0, 0, 0, 42],
            ),
            (
                Fraction::new_raw(256u128, 1u128),
                &[0, 1, 0, 0, 0, 0, 0, 0, 1, 0],
            ),
            (
                Fraction::new_raw(1234u128, 1u128),
                &[0, 1, 0, 0, 0, 0, 0, 0, 4, 210],
            ),
            (
                Fraction::new_raw(1000000000u128, 1u128),
                &[0, 1, 0, 2, 0, 0, 0, 0, 0, 10],
            ),
            (
                Fraction::new_raw(10000000000000000001u128, 10000000000u128),
                &[
                    0, 6, 0, 2, 0, 0, 0, 10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100,
                ],
            ),
            (
                Fraction::new_raw(12345678u128, 1u128),
                &[0, 2, 0, 1, 0, 0, 0, 0, 4, 210, 22, 46],
            ),
            (
                Fraction::new_raw(12345678901234u128, 1u128),
                &[0, 4, 0, 3, 0, 0, 0, 0, 0, 12, 13, 128, 30, 210, 4, 210],
            ),
            (
                Fraction::new_raw(33333333333333333333u128, 100000000000000000000u128),
                &[
                    0, 5, 255, 255, 0, 0, 0, 20, 13, 5, 13, 5, 13, 5, 13, 5, 13, 5,
                ],
            ),
        ]
    }

    #[test]
    fn test_from_sql() {
        let t = Type::from_oid(NUMERIC_OID).unwrap();
        for ref test in &get_tests() {
            assert_eq!(
                test.0,
                <Fraction as FromSql>::from_sql(&t, test.1).ok().unwrap()
            )
        }
    }

    #[test]
    fn test_to_sql() {
        let t = Type::from_oid(NUMERIC_OID).unwrap();
        let mut buf = BytesMut::with_capacity(1024);

        for ref test in &get_tests() {
            buf.clear();
            let res = <Fraction as ToSql>::to_sql(&test.0, &t, &mut buf)
                .ok()
                .unwrap();

            match res {
                IsNull::Yes => assert!(false),
                IsNull::No => assert!(true),
            };

            assert_eq!(&buf, &test.1);
        }
    }

    #[test]
    fn test_to_sql_infinity_appends_after_existing_bytes() {
        let t = Type::from_oid(NUMERIC_OID).unwrap();
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[1u8, 2, 3, 4]);

        let res = <Fraction as ToSql>::to_sql(&Fraction::infinity(), &t, &mut buf)
            .ok()
            .unwrap();

        match res {
            IsNull::Yes => assert!(false),
            IsNull::No => assert!(true),
        };

        assert_eq!(&buf[..4], &[1u8, 2, 3, 4]);
        assert_eq!(&buf[4..], &[0u8, 0, 0, 0, 208, 0, 0, 0]);
    }
}
