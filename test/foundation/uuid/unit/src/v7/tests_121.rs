#[cfg(test)]
mod tests {
    use super::*;

    use crate::{std::string::ToString, ClockSequence, NoContext, Variant, Version};

    #[cfg(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")))]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new() {
        let ts: u64 = 1645557742000;

        let seconds = ts / 1000;
        let nanos = ((ts % 1000) * 1_000_000) as u32;

        let uuid = Uuid::new_v7(Timestamp::from_unix(NoContext, seconds, nanos));
        let uustr = uuid.hyphenated().to_string();

        assert_eq!(uuid.get_version(), Some(Version::SortRand));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
        assert!(uuid.hyphenated().to_string().starts_with("017f22e2-79b0-7"));

        // Ensure parsing the same UUID produces the same timestamp
        let parsed = Uuid::parse_str(uustr.as_str()).unwrap();

        assert_eq!(uuid, parsed);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    #[cfg(feature = "std")]
    fn test_now() {
        let uuid = Uuid::now_v7();

        assert_eq!(uuid.get_version(), Some(Version::SortRand));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_sorting() {
        let time1: u64 = 1_496_854_535;
        let time_fraction1: u32 = 812_000_000;

        let time2 = time1 + 4000;
        let time_fraction2 = time_fraction1;

        let uuid1 = Uuid::new_v7(Timestamp::from_unix(NoContext, time1, time_fraction1));
        let uuid2 = Uuid::new_v7(Timestamp::from_unix(NoContext, time2, time_fraction2));

        assert!(uuid1.as_bytes() < uuid2.as_bytes());
        assert!(uuid1.to_string() < uuid2.to_string());
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new_timestamp_roundtrip() {
        let time: u64 = 1_496_854_535;
        let time_fraction: u32 = 812_000_000;

        let ts = Timestamp::from_unix(NoContext, time, time_fraction);

        let uuid = Uuid::new_v7(ts);

        let decoded_ts = uuid.get_timestamp().unwrap();

        assert_eq!(ts.to_unix(), decoded_ts.to_unix());
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new_max_context() {
        struct MaxContext;

        impl ClockSequence for MaxContext {
            type Output = u128;

            fn generate_sequence(&self, _seconds: u64, _nanos: u32) -> Self::Output {
                u128::MAX
            }

            fn usable_bits(&self) -> usize {
                128
            }
        }

        let time: u64 = 1_496_854_535;
        let time_fraction: u32 = 812_000_000;

        // Ensure we don't overflow here
        let ts = Timestamp::from_unix(MaxContext, time, time_fraction);

        let uuid = Uuid::new_v7(ts);

        assert_eq!(uuid.get_version(), Some(Version::SortRand));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);

        let decoded_ts = uuid.get_timestamp().unwrap();

        assert_eq!(ts.to_unix(), decoded_ts.to_unix());
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new_counter_range() {
        for (width, eq) in [
            (0, false),
            (3, false),
            (43, false),
            (74, true),
            (u8::MAX, true),
        ] {
            for counter in [0u128, u128::MAX] {
                let ts = Timestamp::from_unix_time(1_700_000_000, 0, counter, width);

                let a = Uuid::new_v7(ts);
                let b = Uuid::new_v7(ts);

                assert_eq!((1_700_000_000, 0), a.get_timestamp().unwrap().to_unix());
                assert_eq!((1_700_000_000, 0), b.get_timestamp().unwrap().to_unix());

                assert_eq!(
                    eq,
                    a == b,
                    "{:>032x} = {:>032x} with counter {counter:x} should be {eq:?}",
                    a.as_u128(),
                    b.as_u128()
                );
            }
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_42bit_counter_is_fully_preserved() {
        fn rand_a(uuid: &Uuid) -> u16 {
            let b = uuid.as_bytes();

            (((b[6] & 0x0f) as u16) << 8) | b[7] as u16
        }

        // `rand_a` and the top 30 bits of `rand_b`, with the variant masked out
        fn counter_bits(uuid: &Uuid) -> (u16, [u8; 4]) {
            let b = uuid.as_bytes();

            (rand_a(uuid), [b[8] & 0x3f, b[9], b[10], b[11]])
        }

        for bit in 0..42 {
            let counter = 1u128 << bit;

            let with = Uuid::new_v7(Timestamp::from_unix_time(0, 0, counter, 42));
            let without = Uuid::new_v7(Timestamp::from_unix_time(0, 0, 0, 42));

            assert_ne!(
                counter_bits(&with),
                counter_bits(&without),
                "counter bit {bit} did not reach the UUID"
            );
        }

        let all_ones = Uuid::new_v7(Timestamp::from_unix_time(0, 0, (1 << 42) - 1, 42));
        assert_eq!(0x0fff, rand_a(&all_ones));
        assert_eq!(Variant::RFC4122, all_ones.get_variant());

        let before = Uuid::new_v7(Timestamp::from_unix_time(1, 0, 0x3f_ffff_ffff, 42));
        let after = Uuid::new_v7(Timestamp::from_unix_time(1, 0, 0x40_0000_0000, 42));
        assert!(before < after, "{before} should sort before {after}");
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_additional_precision_preserves_sorting() {
        for start in (0..1_000_000).step_by(50_000) {
            for delta in [50_000u32, 100_000, 200_000] {
                if start + delta >= 1_000_000 {
                    continue;
                }

                let context = crate::ContextV7::new().with_additional_precision();

                let earlier = Uuid::new_v7(Timestamp::from_unix(&context, 1_700_000_000, start));
                let later =
                    Uuid::new_v7(Timestamp::from_unix(&context, 1_700_000_000, start + delta));

                assert!(
                    earlier < later,
                    "{start}ns gave {earlier} and {}ns gave {later}",
                    start + delta
                );
            }
        }
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn test_new_max() {
        let ts = Timestamp::from_unix_time(u64::MAX, 0, 0, 0);
        let uuid = Uuid::new_v7(ts);

        let decoded_ts = uuid.get_timestamp().unwrap();

        assert_eq!((281474976710, 655000000), decoded_ts.to_unix());
    }
}
