#[cfg(test)]
mod tests {
    use super::Set;
    use bincode::data_serialization::{decode_from_slice, encode_into_slice};

    #[test]
    fn serialize_and_deserialize() {
        let config = bincode::config::legacy();
        let mut before: Set<u8, 8> = Set::new();
        before.insert(1);
        let mut bytes: [u8; 1024] = [0; 1024];
        let len = encode_into_slice(&before, &mut bytes, config).unwrap();
        let bytes = &bytes[..len];
        let (after, read_len): (Set<u8, 8>, usize) = decode_from_slice(&bytes, config).unwrap();
        assert_eq!(1, after.into_iter().next().unwrap());
        assert_eq!(bytes.len(), read_len);
    }

    #[test]
    fn empty_set_serialization() {
        let config = bincode::config::legacy();
        let before: Set<u8, 8> = Set::new();
        let mut bytes: [u8; 1024] = [0; 1024];
        let len = encode_into_slice(&before, &mut bytes, config).unwrap();
        let bytes = &bytes[..len];
        let (after, read_len): (Set<u8, 8>, usize) = decode_from_slice(&bytes, config).unwrap();
        assert!(after.is_empty());
        assert_eq!(bytes.len(), read_len);
    }
}
