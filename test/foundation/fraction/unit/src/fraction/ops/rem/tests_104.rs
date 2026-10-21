#[cfg(test)]
mod tests {
    use super::GenericFraction;

    type F = GenericFraction<u8>;

    #[test]
    fn mul_scalar() {
        assert_eq!(F::from(3) % 2, F::from(1));
        assert_eq!(F::from(3) % 1.5, F::from(0));

        assert_eq!(&F::from(3) % 2, F::from(1));
        assert_eq!(&F::from(3) % 1.5, F::from(0));

        assert_eq!(F::from(3) % F::from(2), F::from(1));
        assert_eq!(F::from(3) % F::from(1.5), F::from(0));

        assert_eq!(&F::from(3) % &F::from(2), F::from(1));
        assert_eq!(&F::from(3) % &F::from(1.5), F::from(0));
    }
}
