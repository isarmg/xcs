#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_valid() {
        const PHRASE: &[u8] = b"OK";
        assert_eq!(ReasonPhrase::from_static(PHRASE).as_bytes(), PHRASE);
        assert_eq!(ReasonPhrase::try_from(PHRASE).unwrap().as_bytes(), PHRASE);
    }

    #[test]
    fn empty_valid() {
        const PHRASE: &[u8] = b"";
        assert_eq!(ReasonPhrase::from_static(PHRASE).as_bytes(), PHRASE);
        assert_eq!(ReasonPhrase::try_from(PHRASE).unwrap().as_bytes(), PHRASE);
    }

    #[test]
    fn obs_text_valid() {
        const PHRASE: &[u8] = b"hyp\xe9r";
        assert_eq!(ReasonPhrase::from_static(PHRASE).as_bytes(), PHRASE);
        assert_eq!(ReasonPhrase::try_from(PHRASE).unwrap().as_bytes(), PHRASE);
    }

    const NEWLINE_PHRASE: &[u8] = b"hyp\ner";

    #[test]
    #[should_panic]
    fn newline_invalid_panic() {
        ReasonPhrase::from_static(NEWLINE_PHRASE);
    }

    #[test]
    fn newline_invalid_err() {
        assert!(ReasonPhrase::try_from(NEWLINE_PHRASE).is_err());
    }

    const CR_PHRASE: &[u8] = b"hyp\rer";

    #[test]
    #[should_panic]
    fn cr_invalid_panic() {
        ReasonPhrase::from_static(CR_PHRASE);
    }

    #[test]
    fn cr_invalid_err() {
        assert!(ReasonPhrase::try_from(CR_PHRASE).is_err());
    }
}
