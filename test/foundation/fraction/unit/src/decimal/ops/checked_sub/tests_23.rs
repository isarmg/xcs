#[cfg(test)]
mod tests {
    use super::{CheckedSub, GenericDecimal};
    use crate::{One, Zero};

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn checked_sub() {
        assert_eq!(Some(F::nan()), F::nan().checked_sub(&F::nan()));

        assert_eq!(Some(F::nan()), F::infinity().checked_sub(&F::nan()));
        assert_eq!(Some(F::nan()), F::infinity().checked_sub(&F::infinity()));
        assert_eq!(
            Some(F::infinity()),
            F::infinity().checked_sub(&F::neg_infinity())
        );
        assert_eq!(Some(F::infinity()), F::infinity().checked_sub(&F::one()));

        assert_eq!(Some(F::nan()), F::one().checked_sub(&F::nan()));
        assert_eq!(
            Some(F::neg_infinity()),
            F::one().checked_sub(&F::infinity())
        );
        assert_eq!(
            Some(F::infinity()),
            F::one().checked_sub(&F::neg_infinity())
        );

        assert_eq!(Some(F::zero()), F::one().checked_sub(&F::one()));
        assert_eq!(Some(F::from(2)), F::one().checked_sub(&(-F::one())));
        assert_eq!(Some(-F::from(2)), (-F::one()).checked_sub(&F::one()));
        assert_eq!(Some(F::zero()), (-F::one()).checked_sub(&(-F::one())));

        assert_eq!(Some(-F::one()), F::one().checked_sub(&F::from(2)));
        assert_eq!(Some(F::one()), (-F::one()).checked_sub(&(-F::from(2))));
    }
}
