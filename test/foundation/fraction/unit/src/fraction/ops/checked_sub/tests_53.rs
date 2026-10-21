#[cfg(test)]
mod tests {
    use super::{CheckedSub, GenericFraction};
    use crate::{One, Zero};

    type F = GenericFraction<u8>;

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
        assert_eq!(Some(F::new(2, 1)), F::one().checked_sub(&F::new_neg(1, 1)));
        assert_eq!(
            Some(F::new_neg(2, 1)),
            F::new_neg(1, 1).checked_sub(&F::one())
        );
        assert_eq!(
            Some(F::zero()),
            F::new_neg(1, 1).checked_sub(&F::new_neg(1, 1))
        );

        assert_eq!(Some(F::new_neg(1, 1)), F::one().checked_sub(&F::new(2, 1)));
        assert_eq!(
            Some(F::one()),
            F::new_neg(1, 1).checked_sub(&F::new_neg(2, 1))
        );
    }
}
