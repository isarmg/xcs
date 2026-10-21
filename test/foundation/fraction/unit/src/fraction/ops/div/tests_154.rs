#[cfg(test)]
mod tests {
    use super::GenericFraction;
    // use crate::{One, Zero};

    type F = GenericFraction<u8>;

    #[test]
    fn div_scalar() {
        assert_eq!(F::from(3) / 2, F::from(1.5));
        assert_eq!(F::from(3) / 1.5, F::from(2));

        assert_eq!(&F::from(3) / 2, F::from(1.5));
        assert_eq!(&F::from(3) / 1.5, F::from(2));

        assert_eq!(F::from(3) / F::from(2), F::from(1.5));
        assert_eq!(F::from(3) / F::from(1.5), F::from(2));

        assert_eq!(&F::from(3) / &F::from(2), F::from(1.5));
        assert_eq!(&F::from(3) / &F::from(1.5), F::from(2));
    }
}
