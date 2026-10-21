#[cfg(test)]
mod tests {
    use convert::TryToConvertFrom;
    use fraction::GenericFraction;

    type F1 = GenericFraction<u8>;
    type F2 = GenericFraction<i8>;

    #[test]
    fn try_to_convert_integers() {
        {
            assert_eq!(Some(127i8), i8::try_to_convert_from(127u8));
            assert_eq!(Some(127u8), u8::try_to_convert_from(127i8));
            assert_eq!(None, i8::try_to_convert_from(128u8));
            assert_eq!(None, u8::try_to_convert_from(-127i8));
            assert_eq!(None, u8::try_to_convert_from(256u16));
        }
    }

    #[test]
    fn try_to_convert_generic_fraction() {
        {
            let fu = F1::new(1u8, 2u8);
            let fi = F2::try_to_convert_from(fu);

            assert!(fi.is_some());
            assert_eq!(fi.unwrap(), F2::new(1i8, 2i8));
        }

        {
            let fi = F2::new(1i8, 2i8);
            let fu = F1::try_to_convert_from(fi);

            assert!(fu.is_some());
            assert_eq!(fu.unwrap(), F1::new(1u8, 2u8));
        }

        {
            let fu = F1::infinity();
            let fi = F2::try_to_convert_from(fu);

            assert!(fi.is_some());
            assert_eq!(fi.unwrap(), F2::infinity());
        }

        {
            let fu = F1::neg_infinity();
            let fi = F2::try_to_convert_from(fu);

            assert!(fi.is_some());
            assert_eq!(fi.unwrap(), F2::neg_infinity());
        }
    }
}
