#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! check_to_from_bytes {
        ($( $ty:ty )+) => {$({
            let n = 1;
            let be = <$ty as ToBytes>::to_be_bytes(&n);
            let le = <$ty as ToBytes>::to_le_bytes(&n);
            let ne = <$ty as ToBytes>::to_ne_bytes(&n);

            assert_eq!(*be.last().unwrap(), 1);
            assert_eq!(*le.first().unwrap(), 1);
            if cfg!(target_endian = "big") {
                assert_eq!(*ne.last().unwrap(), 1);
            } else {
                assert_eq!(*ne.first().unwrap(), 1);
            }

            assert_eq!(<$ty as FromBytes>::from_be_bytes(&be), n);
            assert_eq!(<$ty as FromBytes>::from_le_bytes(&le), n);
            if cfg!(target_endian = "big") {
                assert_eq!(<$ty as FromBytes>::from_ne_bytes(&be), n);
            } else {
                assert_eq!(<$ty as FromBytes>::from_ne_bytes(&le), n);
            }
        })+}
    }

    #[test]
    fn convert_between_int_and_bytes() {
        check_to_from_bytes!(u8 u16 u32 u64 u128 usize);
        check_to_from_bytes!(i8 i16 i32 i64 i128 isize);
    }

    #[test]
    fn convert_between_float_and_bytes() {
        macro_rules! check_to_from_bytes {
            ($( $ty:ty )+) => {$(
                let n: $ty = 3.14;

                let be = <$ty as ToBytes>::to_be_bytes(&n);
                let le = <$ty as ToBytes>::to_le_bytes(&n);
                let ne = <$ty as ToBytes>::to_ne_bytes(&n);

                assert_eq!(<$ty as FromBytes>::from_be_bytes(&be), n);
                assert_eq!(<$ty as FromBytes>::from_le_bytes(&le), n);
                if cfg!(target_endian = "big") {
                    assert_eq!(ne, be);
                    assert_eq!(<$ty as FromBytes>::from_ne_bytes(&be), n);
                } else {
                    assert_eq!(ne, le);
                    assert_eq!(<$ty as FromBytes>::from_ne_bytes(&le), n);
                }
            )+}
        }

        check_to_from_bytes!(f32 f64);
    }
}
