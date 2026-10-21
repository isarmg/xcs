#[cfg(test)]
mod tests {
    use super::*;

    use crate::{std::string::ToString, ContextV1, Variant, Version};

    #[cfg(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")))]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new() {
        let time: u64 = 1_496_854_535;
        let time_fraction: u32 = 812_946_000;
        let node = [1, 2, 3, 4, 5, 6];
        let context = ContextV1::new(0);

        let uuid = Uuid::new_v1(Timestamp::from_unix(&context, time, time_fraction), &node);

        assert_eq!(uuid.get_version(), Some(Version::Mac));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
        assert_eq!(
            uuid.hyphenated().to_string(),
            "20616934-4ba2-11e7-8000-010203040506"
        );

        let ts = uuid.get_timestamp().unwrap().to_gregorian();

        assert_eq!(ts.0 - 0x01B2_1DD2_1381_4000, 14_968_545_358_129_460);

        assert_eq!(Some(node), uuid.get_node_id());

        // Ensure parsing the same UUID produces the same timestamp
        let parsed = Uuid::parse_str("20616934-4ba2-11e7-8000-010203040506").unwrap();

        assert_eq!(
            uuid.get_timestamp().unwrap(),
            parsed.get_timestamp().unwrap()
        );

        assert_eq!(uuid.get_node_id().unwrap(), parsed.get_node_id().unwrap());
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    #[cfg(all(feature = "std", feature = "rng"))]
    fn test_now() {
        let node = [1, 2, 3, 4, 5, 6];

        let uuid = Uuid::now_v1(&node);

        assert_eq!(uuid.get_version(), Some(Version::Mac));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
    }
}
