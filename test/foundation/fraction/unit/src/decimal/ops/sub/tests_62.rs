#[cfg(test)]
mod tests {
    use crate::{GenericDecimal, One, Zero};

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn sub_scalar() {
        assert_eq!(F::one() - 1, F::zero());
        assert_eq!(F::one() - 0.5, F::from(0.5));

        assert_eq!(&F::one() - 1, F::zero());
        assert_eq!(&F::one() - 0.5, F::from(0.5));

        assert_eq!(F::one() - F::from(1), F::zero());
        assert_eq!(F::one() - F::from(0.5), F::from(0.5));

        assert_eq!(&F::one() - &F::from(1), F::zero());
        assert_eq!(&F::one() - &F::from(0.5), F::from(0.5));
    }
}
