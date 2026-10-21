#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn debug_names() {
        assert_eq!(
            format!(
                "{:?}",
                GeneralName::DnsName(untrusted::Input::from(b"example.com"))
            ),
            "DnsName(\"example.com\")"
        );

        assert_eq!(format!("{:?}", GeneralName::DirectoryName), "DirectoryName");

        assert_eq!(
            format!(
                "{:?}",
                GeneralName::IpAddress(untrusted::Input::from(&[192, 0, 2, 1][..]))
            ),
            "IpAddress(192.0.2.1)"
        );

        assert_eq!(
            format!(
                "{:?}",
                GeneralName::IpAddress(untrusted::Input::from(
                    &[0x20, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x0d, 0xb8][..]
                ))
            ),
            "IpAddress(2001::db8)"
        );

        assert_eq!(
            format!(
                "{:?}",
                GeneralName::IpAddress(untrusted::Input::from(&[1, 2, 3, 4, 5, 6][..]))
            ),
            "IpAddress([invalid: 01, 02, 03, 04, 05, 06])"
        );

        assert_eq!(
            format!(
                "{:?}",
                GeneralName::UniformResourceIdentifier(untrusted::Input::from(
                    b"https://example.com"
                ))
            ),
            "UniformResourceIdentifier(\"https://example.com\")"
        );

        assert_eq!(
            format!("{:?}", GeneralName::Unsupported(0x66)),
            "Unsupported(0x66)"
        );
    }

    #[test]
    fn name_iter_end_after_error() {
        let input = untrusted::Input::from(&[0x30]);
        let mut iter = NameIterator::new(Some(input));
        assert_eq!(iter.next().unwrap().unwrap_err(), Error::BadDer);
        assert!(iter.next().is_none());
    }
}
