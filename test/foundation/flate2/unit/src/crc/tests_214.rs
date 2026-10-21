#[cfg(test)]
mod tests {
    use super::Crc;

    fn crc_of(data: &[u8]) -> Crc {
        let mut c = Crc::new();
        c.update(data);
        c
    }

    fn sum_of(data: &[u8]) -> u32 {
        crc_of(data).sum()
    }

    #[test]
    fn new_is_empty() {
        let c = Crc::new();
        assert_eq!(c.amount(), 0);
        assert_eq!(c.sum(), 0);
    }

    #[test]
    fn known_vector_hello() {
        assert_eq!(sum_of(b"hello"), 0x3610_A686);
    }

    #[test]
    fn known_vector_quick_brown_fox() {
        assert_eq!(
            sum_of(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
    }

    #[test]
    fn update_is_streaming() {
        let mut c = Crc::new();
        c.update(b"hello");
        c.update(b" ");
        c.update(b"world");

        assert_eq!(c.amount(), 11);
        assert_eq!(c.sum(), sum_of(b"hello world"));
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut c = Crc::new();
        c.update(b"abc");
        assert_ne!(c.sum(), 0);
        assert_eq!(c.amount(), 3);

        c.reset();
        assert_eq!(c.amount(), 0);
        assert_eq!(c.sum(), 0);
    }

    #[test]
    fn combine_matches_concatenation() {
        let a = b"hello ";
        let b = b"world";

        let mut ca = crc_of(a);
        let cb = crc_of(b);

        ca.combine(&cb);

        assert_eq!(ca.amount(), 11);
        assert_eq!(ca.sum(), sum_of(b"hello world"));
    }
}
