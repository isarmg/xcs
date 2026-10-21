#[cfg(test)]
mod tests {
    use super::{Error, Ordering, Output};

    #[test]
    fn new_with_valid_min_length_input() {
        let bytes = [10u8; 10];
        let output = Output::new(&bytes).unwrap();
        assert_eq!(output.as_ref(), &bytes);
    }

    #[test]
    fn new_with_valid_max_length_input() {
        let bytes = [64u8; 64];
        let output = Output::new(&bytes).unwrap();
        assert_eq!(output.as_ref(), &bytes);
    }

    #[test]
    fn reject_new_too_short() {
        let bytes = [9u8; 9];
        let err = Output::new(&bytes).err().unwrap();
        assert_eq!(
            err,
            Error::OutputSize {
                provided: Ordering::Less,
                expected: Output::MIN_LENGTH
            }
        );
    }

    #[test]
    fn reject_new_too_long() {
        let bytes = [65u8; 65];
        let err = Output::new(&bytes).err().unwrap();
        assert_eq!(
            err,
            Error::OutputSize {
                provided: Ordering::Greater,
                expected: Output::MAX_LENGTH
            }
        );
    }

    #[test]
    fn partialeq_true() {
        let a = Output::new(&[1u8; 32]).unwrap();
        let b = Output::new(&[1u8; 32]).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn partialeq_false() {
        let a = Output::new(&[1u8; 32]).unwrap();
        let b = Output::new(&[2u8; 32]).unwrap();
        assert_ne!(a, b);
    }
}
