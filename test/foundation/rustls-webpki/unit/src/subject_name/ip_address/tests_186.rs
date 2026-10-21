#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presented_id_matches_constraint_ipv4_test() {
        let names_and_constraints = vec![
            (
                // 192.0.2.0 matches constraint 192.0.2.0/24
                [0xC0, 0x00, 0x02, 0x00],
                [0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00],
                Ok(true),
            ),
            (
                // 192.0.2.1 matches constraint 192.0.2.0/24
                [0xC0, 0x00, 0x02, 0x01],
                [0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00],
                Ok(true),
            ),
            (
                // 192.0.2.255 matches constraint 192.0.2.0/24
                [0xC0, 0x00, 0x02, 0xFF],
                [0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00],
                Ok(true),
            ),
            (
                // 192.0.1.255 does not match constraint 192.0.2.0/24
                [0xC0, 0x00, 0x01, 0xFF],
                [0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00],
                Ok(false),
            ),
            (
                // 192.0.3.0 does not match constraint 192.0.2.0/24
                [0xC0, 0x00, 0x03, 0x00],
                [0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00],
                Ok(false),
            ),
        ];
        for (name, constraint, match_result) in names_and_constraints {
            assert_eq!(
                presented_id_matches_constraint(
                    untrusted::Input::from(&name),
                    untrusted::Input::from(&constraint),
                ),
                match_result
            )
        }

        // Invalid name length (shorter)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[0xC0, 0x00, 0x02]),
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00]),
            ),
            Err(Error::BadDer),
        );

        // Invalid name length (longer)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00, 0x00]),
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00]),
            ),
            Err(Error::BadDer),
        );

        // Unmatching constraint size (shorter)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00]),
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF]),
            ),
            Err(Error::InvalidNetworkMaskConstraint),
        );

        // Unmatching constraint size (longer)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00]),
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00]),
            ),
            Err(Error::InvalidNetworkMaskConstraint),
        );

        // Unmatching constraint size (IPv6 constraint for IPv4 address)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00]),
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00
                ]),
            ),
            Ok(false),
        );
    }

    #[test]
    fn presented_id_matches_constraint_ipv6_test() {
        let names_and_constraints = vec![
            (
                // 2001:0DB8:ABCD:0012:0000:0000:0000:0000 matches constraint
                //   2001:0DB8:ABCD:0012:0000:0000:0000:0000/64
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                Ok(true),
            ),
            (
                // 2001:0DB8:ABCD:0012:0000:0000:0000:0001 matches constraint
                //   2001:0DB8:ABCD:0012:0000:0000:0000:0000/64
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x01,
                ],
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                Ok(true),
            ),
            (
                // 2001:0DB8:ABCD:0012:FFFF:FFFF:FFFF:FFFF matches constraint
                //   2001:0DB8:ABCD:0012:0000:0000:0000:0000/64
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                    0xFF, 0xFF, 0xFF,
                ],
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                Ok(true),
            ),
            (
                // 2001:0DB8:ABCD:0011:0000:0000:0000:0000 does not match constraint
                //   2001:0DB8:ABCD:0012:0000:0000:0000:0000/64
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                Ok(false),
            ),
            (
                // 2001:0DB8:ABCD:0013:0000:0000:0000:0000 does not match constraint
                //   2001:0DB8:ABCD:0012:0000:0000:0000:0000/64
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ],
                [
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ],
                Ok(false),
            ),
        ];
        for (name, constraint, match_result) in names_and_constraints {
            assert_eq!(
                presented_id_matches_constraint(
                    untrusted::Input::from(&name),
                    untrusted::Input::from(&constraint),
                ),
                match_result
            )
        }

        // Invalid name length (shorter)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00
                ]),
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00
                ]),
            ),
            Err(Error::BadDer),
        );

        // Invalid name length (longer)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00
                ]),
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00
                ]),
            ),
            Err(Error::BadDer),
        );

        // Unmatching constraint size (shorter)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ]),
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00
                ]),
            ),
            Err(Error::InvalidNetworkMaskConstraint),
        );

        // Unmatching constraint size (longer)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ]),
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
                ]),
            ),
            Err(Error::InvalidNetworkMaskConstraint),
        );

        // Unmatching constraint size (IPv4 constraint for IPv6 address)
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(&[
                    0x20, 0x01, 0x0D, 0xB8, 0xAB, 0xCD, 0x00, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00,
                ]),
                untrusted::Input::from(&[0xC0, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0x00]),
            ),
            Ok(false),
        );
    }

    #[test]
    fn test_presented_id_matches_reference_id() {
        assert!(!presented_id_matches_reference_id(
            untrusted::Input::from(&[]),
            untrusted::Input::from(&[]),
        ));

        assert!(!presented_id_matches_reference_id(
            untrusted::Input::from(&[0x01]),
            untrusted::Input::from(&[])
        ));

        assert!(!presented_id_matches_reference_id(
            untrusted::Input::from(&[]),
            untrusted::Input::from(&[0x01])
        ));

        assert!(presented_id_matches_reference_id(
            untrusted::Input::from(&[1, 2, 3, 4]),
            untrusted::Input::from(&[1, 2, 3, 4])
        ));

        assert!(!presented_id_matches_reference_id(
            untrusted::Input::from(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            untrusted::Input::from(&[1, 2, 3, 4])
        ));

        assert!(!presented_id_matches_reference_id(
            untrusted::Input::from(&[1, 2, 3, 4]),
            untrusted::Input::from(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
        ));

        assert!(presented_id_matches_reference_id(
            untrusted::Input::from(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            untrusted::Input::from(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
        ));
    }

    #[test]
    fn presented_id_matches_constraint_rejects_incorrect_length_arguments() {
        // wrong length names
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(b"\x00\x00\x00"),
                untrusted::Input::from(b"")
            ),
            Err(Error::BadDer)
        );
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(b"\x00\x00\x00\x00\x00"),
                untrusted::Input::from(b"")
            ),
            Err(Error::BadDer)
        );

        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(
                    b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
                ),
                untrusted::Input::from(b"")
            ),
            Err(Error::BadDer)
        );
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(
                    b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
                ),
                untrusted::Input::from(b"")
            ),
            Err(Error::BadDer)
        );

        // wrong length constraints
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(b"\x00\x00\x00\x00"),
                untrusted::Input::from(b"\x00\x00\x00\x00\xff\xff\xff")
            ),
            Err(Error::InvalidNetworkMaskConstraint)
        );
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(b"\x00\x00\x00\x00"),
                untrusted::Input::from(b"\x00\x00\x00\x00\xff\xff\xff\xff\x00")
            ),
            Err(Error::InvalidNetworkMaskConstraint)
        );
        assert_eq!(
            presented_id_matches_constraint(untrusted::Input::from(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"),
                                            untrusted::Input::from(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
                                                                     \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff")),
            Err(Error::InvalidNetworkMaskConstraint)
        );
        assert_eq!(
            presented_id_matches_constraint(untrusted::Input::from(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"),
                                            untrusted::Input::from(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
                                                                     \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff")),
            Err(Error::InvalidNetworkMaskConstraint)
        );

        // ipv4-length not considered for ipv6-length name, and vv
        assert_eq!(
            presented_id_matches_constraint(untrusted::Input::from(b"\x00\x00\x00\x00"),
                                            untrusted::Input::from(b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\
                                                                     \xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff")),
            Ok(false)
        );
        assert_eq!(
            presented_id_matches_constraint(
                untrusted::Input::from(
                    b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
                ),
                untrusted::Input::from(b"\x00\x00\x00\x00\xff\xff\xff\xff")
            ),
            Ok(false)
        );
    }
}
