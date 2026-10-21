#[cfg(test)]
mod tests {
    use super::*;

    use core::mem::MaybeUninit;

    #[test]
    fn encode_lower_uninit() {
        let uuid = Uuid::parse_str("936DA01f9abd4d9d80c702af85c822a8").unwrap();

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.hyphenated().encode_lower_uninit(&mut buf);
            assert_eq!(encoded, "936da01f-9abd-4d9d-80c7-02af85c822a8");
            encoded.len()
        };
        assert_eq!(len, Hyphenated::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.simple().encode_lower_uninit(&mut buf);
            assert_eq!(encoded, "936da01f9abd4d9d80c702af85c822a8");
            encoded.len()
        };
        assert_eq!(len, Simple::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.urn().encode_lower_uninit(&mut buf);
            assert_eq!(encoded, "urn:uuid:936da01f-9abd-4d9d-80c7-02af85c822a8");
            encoded.len()
        };
        assert_eq!(len, Urn::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.braced().encode_lower_uninit(&mut buf);
            assert_eq!(encoded, "{936da01f-9abd-4d9d-80c7-02af85c822a8}");
            encoded.len()
        };
        assert_eq!(len, Braced::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);
    }

    #[test]
    fn encode_upper_uninit() {
        let uuid = Uuid::parse_str("936da01f9abd4d9d80c702af85c822a8").unwrap();

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.hyphenated().encode_upper_uninit(&mut buf);
            assert_eq!(encoded, "936DA01F-9ABD-4D9D-80C7-02AF85C822A8");
            encoded.len()
        };
        assert_eq!(len, Hyphenated::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.simple().encode_upper_uninit(&mut buf);
            assert_eq!(encoded, "936DA01F9ABD4D9D80C702AF85C822A8");
            encoded.len()
        };
        assert_eq!(len, Simple::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.urn().encode_upper_uninit(&mut buf);
            assert_eq!(encoded, "urn:uuid:936DA01F-9ABD-4D9D-80C7-02AF85C822A8");
            encoded.len()
        };
        assert_eq!(len, Urn::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);

        let mut buf = [MaybeUninit::new(b'x'); 100];
        let len = {
            let encoded = uuid.braced().encode_upper_uninit(&mut buf);
            assert_eq!(encoded, "{936DA01F-9ABD-4D9D-80C7-02AF85C822A8}");
            encoded.len()
        };
        assert_eq!(len, Braced::LENGTH);
        assert_uninit_tail_unchanged(&buf, len);
    }

    fn assert_uninit_tail_unchanged(buffer: &[MaybeUninit<u8>], initialized: usize) {
        // SAFETY: The test initializes the full buffer before encoding.
        let buffer: &[u8] =
            unsafe { core::slice::from_raw_parts(buffer.as_ptr().cast(), buffer.len()) };

        assert!(buffer[initialized..].iter().all(|b| *b == b'x'));
    }

    #[test]
    fn hyphenated_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().hyphenated().encode_lower(&mut buf).len();
        assert_eq!(len, super::Hyphenated::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn hyphenated_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_hyphenated().encode_lower(&mut buf).len();
        assert_eq!(len, super::Hyphenated::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn simple_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().simple().encode_lower(&mut buf).len();
        assert_eq!(len, super::Simple::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn simple_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_simple().encode_lower(&mut buf).len();
        assert_eq!(len, super::Simple::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn urn_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().urn().encode_lower(&mut buf).len();
        assert_eq!(len, super::Urn::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn urn_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_urn().encode_lower(&mut buf).len();
        assert_eq!(len, super::Urn::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn braced_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().braced().encode_lower(&mut buf).len();
        assert_eq!(len, super::Braced::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    fn braced_ref_trailing() {
        let mut buf = [b'x'; 100];
        let len = Uuid::nil().as_braced().encode_lower(&mut buf).len();
        assert_eq!(len, super::Braced::LENGTH);
        assert!(buf[len..].iter().all(|x| *x == b'x'));
    }

    #[test]
    #[should_panic]
    fn hyphenated_too_small() {
        Uuid::nil().hyphenated().encode_lower(&mut [0; 35]);
    }

    #[test]
    #[should_panic]
    fn simple_too_small() {
        Uuid::nil().simple().encode_lower(&mut [0; 31]);
    }

    #[test]
    #[should_panic]
    fn urn_too_small() {
        Uuid::nil().urn().encode_lower(&mut [0; 44]);
    }

    #[test]
    #[should_panic]
    fn braced_too_small() {
        Uuid::nil().braced().encode_lower(&mut [0; 37]);
    }

    #[test]
    fn hyphenated_to_inner() {
        let hyphenated = Uuid::nil().hyphenated();
        assert_eq!(Uuid::from(hyphenated), Uuid::nil());
    }

    #[test]
    fn simple_to_inner() {
        let simple = Uuid::nil().simple();
        assert_eq!(Uuid::from(simple), Uuid::nil());
    }

    #[test]
    fn urn_to_inner() {
        let urn = Uuid::nil().urn();
        assert_eq!(Uuid::from(urn), Uuid::nil());
    }

    #[test]
    fn braced_to_inner() {
        let braced = Uuid::nil().braced();
        assert_eq!(Uuid::from(braced), Uuid::nil());
    }
}
