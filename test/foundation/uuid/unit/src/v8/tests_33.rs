#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Variant, Version};
    use std::string::ToString;

    #[cfg(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")))]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new() {
        let buf: [u8; 16] = [
            0xf, 0xe, 0xd, 0xc, 0xb, 0xa, 0x9, 0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1, 0x0,
        ];
        let uuid = Uuid::new_v8(buf);
        assert_eq!(uuid.get_version(), Some(Version::Custom));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
        assert_eq!(uuid.get_version_num(), 8);
        assert_eq!(
            uuid.hyphenated().to_string(),
            "0f0e0d0c-0b0a-8908-8706-050403020100"
        );
    }
}
