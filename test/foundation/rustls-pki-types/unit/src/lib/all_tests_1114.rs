#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn der_debug() {
        let der = Der::from_slice(&[0x01, 0x02, 0x03]);
        assert_eq!(format!("{der:?}"), "0x010203");
    }

    #[test]
    fn alg_id_debug() {
        let alg_id = AlgorithmIdentifier::from_slice(&[0x01, 0x02, 0x03]);
        assert_eq!(format!("{alg_id:?}"), "0x010203");
    }

    #[test]
    fn fips_status_debug() {
        let fips = FipsStatus::certified("hello");
        assert_eq!(format!("{fips:?}"), "Certified { certificate: \"hello\" }");
    }

    #[test]
    fn bytes_inner_equality() {
        let owned_a = BytesInner::Owned(vec![1, 2, 3]);
        let owned_b = BytesInner::Owned(vec![4, 5]);
        let borrowed_a = BytesInner::Borrowed(&[1, 2, 3]);
        let borrowed_b = BytesInner::Borrowed(&[99]);

        // Self-equality.
        assert_eq!(owned_a, owned_a);
        assert_eq!(owned_b, owned_b);
        assert_eq!(borrowed_a, borrowed_a);
        assert_eq!(borrowed_b, borrowed_b);

        // Borrowed vs Owned equality
        assert_eq!(owned_a, borrowed_a);
        assert_eq!(borrowed_a, owned_a);

        // Owned inequality
        assert_ne!(owned_a, owned_b);
        assert_ne!(owned_b, owned_a);

        // Borrowed inequality
        assert_ne!(borrowed_a, borrowed_b);
        assert_ne!(borrowed_b, borrowed_a);

        // Borrowed vs Owned inequality
        assert_ne!(owned_a, borrowed_b);
        assert_ne!(borrowed_b, owned_a);
    }
}
