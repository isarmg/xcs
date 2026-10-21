#[cfg(test)]
mod tests {
    use super::{CheckedAdd, GenericDecimal};
    use crate::{One, Zero};

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn checked_add() {
        assert_eq!(Some(F::nan()), F::nan().checked_add(&F::nan()));

        assert_eq!(Some(F::nan()), F::infinity().checked_add(&F::nan()));
        assert_eq!(
            Some(F::infinity()),
            F::infinity().checked_add(&F::infinity())
        );
        assert_eq!(
            Some(F::nan()),
            F::infinity().checked_add(&F::neg_infinity())
        );
        assert_eq!(Some(F::infinity()), F::infinity().checked_add(&F::one()));

        assert_eq!(Some(F::nan()), F::one().checked_add(&F::nan()));
        assert_eq!(Some(F::infinity()), F::one().checked_add(&F::infinity()));
        assert_eq!(
            Some(F::neg_infinity()),
            F::one().checked_add(&F::neg_infinity())
        );

        assert_eq!(Some(F::from(2)), F::one().checked_add(&F::one()));
        assert_eq!(Some(F::zero()), F::one().checked_add(&(-F::one())));
        assert_eq!(Some(F::zero()), (-F::one()).checked_add(&F::one()));
        assert_eq!(Some(-F::from(2)), (-F::one()).checked_add(&(-F::one())));

        assert_eq!(Some(-F::one()), F::one().checked_add(&(-F::from(2))));
        assert_eq!(Some(-F::one()), (-F::from(2)).checked_add(&F::one()));
    }
}
