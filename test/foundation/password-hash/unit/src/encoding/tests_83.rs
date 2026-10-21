#[cfg(test)]
mod tests {
    use super::Encoding;

    #[test]
    fn legacy_crypt_known_vectors_remain_byte_compatible() {
        let vectors: &[(&[u8], &str)] = &[
            (b"", ""),
            (b"\0", ".."),
            (b"***", "8Wce"),
            (b"\x01\x02\x03\x04", ".E61/."),
            (b"\xAD\xAD\xAD\xAD\xAD", "fOqhfOo"),
            (b"\xFF\xEF\xFE\xFF\xEF\xFE", "zyzyzyzy"),
            (b"\xFF\xFF\xFF\xFF\xFF", "zzzzzzw"),
        ];
        for &(raw, encoded) in vectors {
            let mut output = [0u8; 64];
            assert_eq!(Encoding::Crypt.encode(raw, &mut output).unwrap(), encoded);
            assert_eq!(Encoding::Crypt.encoded_len(raw), encoded.len());
            assert_eq!(Encoding::Crypt.decode(encoded, &mut output).unwrap(), raw);
        }
        let mut output = [0u8; 64];
        assert!(Encoding::Crypt.decode("8Wce=", &mut output).is_err());
        assert!(Encoding::Crypt.decode("8Wce\n", &mut output).is_err());
    }
}
