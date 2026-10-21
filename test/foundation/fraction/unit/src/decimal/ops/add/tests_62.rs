#[cfg(test)]
mod tests {
    use super::GenericDecimal;
    use crate::Zero;

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn add_scalar() {
        assert_eq!(F::zero() + 1, F::from(1));
        assert_eq!(F::zero() + 1.5, F::from(1.5));

        assert_eq!(&F::zero() + 1, F::from(1));
        assert_eq!(&F::zero() + 1.5, F::from(1.5));

        assert_eq!(F::zero() + F::from(1), F::from(1));
        assert_eq!(F::zero() + F::from(1.5), F::from(1.5));

        assert_eq!(&F::zero() + F::from(1), F::from(1));
        assert_eq!(&F::zero() + F::from(1.5), F::from(1.5));
    }
}
