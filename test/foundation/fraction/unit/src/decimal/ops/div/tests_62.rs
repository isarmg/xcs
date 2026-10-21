#[cfg(test)]
mod tests {
    use super::GenericDecimal;
    type F = GenericDecimal<u8, u8>;

    #[test]
    fn div_scalar() {
        {
            let result = F::from(3) / 2;
            assert_eq!(result.get_precision(), 0);
            assert_eq!(result.set_precision(1), F::from(1.5));
        }
        assert_eq!(F::from(3) / 1.5, F::from(2));

        {
            let result = &F::from(3) / 2;
            assert_eq!(result.get_precision(), 0);
            assert_eq!(result.set_precision(1), F::from(1.5));
        }
        assert_eq!(&F::from(3) / 1.5, F::from(2));

        {
            let result = F::from(3) / F::from(2);
            assert_eq!(result.get_precision(), 0);
            assert_eq!(result.set_precision(1), F::from(1.5));
        }
        assert_eq!(F::from(3) / F::from(1.5), F::from(2));

        {
            let result = &F::from(3) / &F::from(2);
            assert_eq!(result.get_precision(), 0);
            assert_eq!(result.set_precision(1), F::from(1.5));
        }
        assert_eq!(&F::from(3) / &F::from(1.5), F::from(2));
    }
}
