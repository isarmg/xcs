#[cfg(test)]
mod tests {
    use super::{CheckedAdd, GenericFraction};
    use crate::{One, Zero};

    type F = GenericFraction<u8>;

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

        assert_eq!(Some(F::new(2, 1)), F::one().checked_add(&F::one()));
        assert_eq!(Some(F::zero()), F::one().checked_add(&F::new_neg(1, 1)));
        assert_eq!(Some(F::zero()), F::new_neg(1, 1).checked_add(&F::one()));
        assert_eq!(
            Some(F::new_neg(2, 1)),
            F::new_neg(1, 1).checked_add(&F::new_neg(1, 1))
        );

        assert_eq!(
            Some(F::new_neg(1, 1)),
            F::one().checked_add(&F::new_neg(2, 1))
        );
        assert_eq!(
            Some(F::new_neg(1, 1)),
            F::new_neg(2, 1).checked_add(&F::one())
        );
    }
}
