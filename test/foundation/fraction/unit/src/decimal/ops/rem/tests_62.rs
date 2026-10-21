#[cfg(test)]
mod tests {
    use super::GenericDecimal;

    type F = GenericDecimal<u8, u8>;

    #[test]
    fn rem_scalar() {
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
