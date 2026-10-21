#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")] // for UnixTime::now
    #[test]
    fn serversessionvalue_is_debug() {
        use std::{format, println, vec};
        let ssv = ServerSessionValue::new(
            None,
            ProtocolVersion::TLSv1_3,
            CipherSuite::TLS13_AES_128_GCM_SHA256,
            &[0xa5; 48],
            None,
            None,
            vec![4, 5, 6],
            UnixTime::now(),
            0x12345678,
        );
        let debug = format!("{ssv:?}");
        println!("{debug}");
        assert!(debug.contains("application_data: 040506"));
        assert!(!debug.contains("a5a5"), "{debug}");
    }

    #[test]
    fn serversessionvalue_no_sni() {
        let bytes = [
            0x00, 0x03, 0x03, 0xc0, 0x23, 0x03, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x12, 0x23, 0x34, 0x45, 0x56, 0x67, 0x78, 0x89, 0xfe, 0xed, 0xf0, 0x0d,
        ];
        let mut rd = Reader::init(&bytes);
        let ssv = ServerSessionValue::read(&mut rd).unwrap();
        assert_eq!(ssv.get_encoding(), bytes);
    }

    #[test]
    fn serversessionvalue_with_cert() {
        let bytes = [
            0x00, 0x03, 0x03, 0xc0, 0x23, 0x03, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x12, 0x23, 0x34, 0x45, 0x56, 0x67, 0x78, 0x89, 0xfe, 0xed, 0xf0, 0x0d,
        ];
        let mut rd = Reader::init(&bytes);
        let ssv = ServerSessionValue::read(&mut rd).unwrap();
        assert_eq!(ssv.get_encoding(), bytes);
    }
}
