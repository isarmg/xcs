#[cfg(test)]
mod tests {
    use super::{CheckedDiv, GenericDecimal};
    use crate::{One, Zero};

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn checked_div() {
        assert_eq!(Some(F::nan()), F::nan().checked_div(&F::nan()));

        assert_eq!(Some(F::nan()), F::infinity().checked_div(&F::nan()));
        assert_eq!(Some(F::nan()), F::infinity().checked_div(&F::infinity()));
        assert_eq!(
            Some(F::nan()),
            F::infinity().checked_div(&F::neg_infinity())
        );
        assert_eq!(Some(F::infinity()), F::infinity().checked_div(&F::one()));
        assert_eq!(
            Some(F::neg_infinity()),
            F::infinity().checked_div(&(-F::one()))
        );

        assert_eq!(Some(F::infinity()), F::infinity().checked_div(&F::zero()));
        assert_eq!(Some(F::zero()), F::zero().checked_div(&F::infinity()));

        assert_eq!(
            Some(F::neg_infinity()),
            F::infinity().checked_div(&(-F::one()))
        );

        assert_eq!(Some(F::nan()), F::one().checked_div(&F::nan()));
        assert_eq!(Some(F::zero()), F::one().checked_div(&F::infinity()));
        assert_eq!(Some(F::zero()), F::one().checked_div(&F::neg_infinity()));

        assert_eq!(Some(F::one()), F::one().checked_div(&F::one()));
        assert_eq!(Some(-F::one()), F::one().checked_div(&(-F::one())));
        assert_eq!(Some(-F::one()), (-F::one()).checked_div(&F::one()));
        assert_eq!(Some(F::one()), (-F::one()).checked_div(&(-F::one())));

        let result = F::one().checked_div(&F::from(2)).unwrap();
        assert_eq!(result.get_precision(), 0);
        assert_eq!(result.set_precision(1), F::from(0.5));

        let result = (-F::one()).checked_div(&(-F::from(2))).unwrap();
        assert_eq!(result.get_precision(), 0);
        assert_eq!(result.set_precision(1), F::from(0.5));

        assert_eq!(Some(F::infinity()), F::one().checked_div(&F::zero()));
        assert_eq!(Some(F::neg_infinity()), (-F::one()).checked_div(&F::zero()));
        assert_eq!(Some(F::zero()), F::zero().checked_div(&F::one()));
        assert_eq!(Some(F::zero()), F::zero().checked_div(&(-F::one())));

        assert_eq!(Some(F::nan()), F::zero().checked_div(&F::zero()));
    }
}
