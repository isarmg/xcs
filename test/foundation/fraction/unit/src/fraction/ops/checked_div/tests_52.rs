#[cfg(test)]
mod tests {
    use super::{CheckedDiv, GenericFraction};
    use crate::{One, Zero};

    type F = GenericFraction<u8>;

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
            F::infinity().checked_div(&F::new_neg(1, 1))
        );

        assert_eq!(Some(F::infinity()), F::infinity().checked_div(&F::zero()));
        assert_eq!(Some(F::zero()), F::zero().checked_div(&F::infinity()));

        assert_eq!(
            Some(F::neg_infinity()),
            F::infinity().checked_div(&F::new_neg(1, 1))
        );

        assert_eq!(Some(F::nan()), F::one().checked_div(&F::nan()));
        assert_eq!(Some(F::zero()), F::one().checked_div(&F::infinity()));
        assert_eq!(Some(F::zero()), F::one().checked_div(&F::neg_infinity()));

        assert_eq!(Some(F::one()), F::one().checked_div(&F::one()));
        assert_eq!(
            Some(F::new_neg(1, 1)),
            F::one().checked_div(&F::new_neg(1, 1))
        );
        assert_eq!(
            Some(F::new_neg(1, 1)),
            F::new_neg(1, 1).checked_div(&F::one())
        );
        assert_eq!(
            Some(F::one()),
            F::new_neg(1, 1).checked_div(&F::new_neg(1, 1))
        );

        assert_eq!(Some(F::new(1, 2)), F::one().checked_div(&F::new(2, 1)));
        assert_eq!(
            Some(F::new(1, 2)),
            F::new_neg(1, 1).checked_div(&F::new_neg(2, 1))
        );

        assert_eq!(Some(F::infinity()), F::one().checked_div(&F::zero()));
        assert_eq!(
            Some(F::neg_infinity()),
            F::new_neg(1, 1).checked_div(&F::zero())
        );
        assert_eq!(Some(F::zero()), F::zero().checked_div(&F::one()));
        assert_eq!(Some(F::zero()), F::zero().checked_div(&F::new_neg(1, 1)));

        assert_eq!(Some(F::nan()), F::zero().checked_div(&F::zero()));
    }
}
