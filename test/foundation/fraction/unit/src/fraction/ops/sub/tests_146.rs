#[cfg(test)]
mod tests {
    use super::GenericFraction;
    use crate::{One, Zero};

    type F = GenericFraction<u8>;

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
