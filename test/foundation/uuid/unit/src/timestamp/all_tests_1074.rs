#[cfg(all(test, any(feature = "v1", feature = "v6")))]
mod tests {
    use super::*;

    #[cfg(all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")))]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn gregorian_unix_does_not_panic() {
        // Ensure timestamp conversions never panic
        Timestamp::unix_to_gregorian_ticks(u64::MAX, 0);
        Timestamp::unix_to_gregorian_ticks(0, u32::MAX);
        Timestamp::unix_to_gregorian_ticks(u64::MAX, u32::MAX);

        Timestamp::gregorian_to_unix(u64::MAX);
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn to_gregorian_truncates_to_usable_bits() {
        let ts = Timestamp::from_gregorian_time(123, u16::MAX);

        assert_eq!((123, u16::MAX >> 2), ts.to_gregorian());
    }

    #[test]
    #[cfg_attr(
        all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
        wasm_bindgen_test
    )]
    fn clock_sequence_usable_bits() {
        struct MyContext;

        impl ClockSequence for MyContext {
            type Output = u16;

            fn generate_sequence(&self, _: u64, _: u32) -> Self::Output {
                0
            }
        }

        assert_eq!(16, MyContext.usable_bits());
    }

    #[cfg(all(test, feature = "std", not(miri)))]
    mod std_support {
        use super::*;

        use std::time::{Duration, SystemTime};

        // Components of an arbitrary timestamp with non-zero nanoseconds.
        const KNOWN_SECONDS: u64 = 1_501_520_400;
        const KNOWN_NANOS: u32 = 1_000;

        fn known_system_time() -> SystemTime {
            SystemTime::UNIX_EPOCH
                .checked_add(Duration::new(KNOWN_SECONDS, KNOWN_NANOS))
                .unwrap()
        }

        fn known_timestamp() -> Timestamp {
            Timestamp::from_unix_time(KNOWN_SECONDS, KNOWN_NANOS, 0, 0)
        }

        #[test]
        fn to_system_time() {
            let st: SystemTime = known_timestamp().into();

            assert_eq!(known_system_time(), st);
        }

        #[test]
        fn from_system_time() {
            let ts: Timestamp = known_system_time().try_into().unwrap();

            assert_eq!(known_timestamp(), ts);
        }

        #[test]
        fn from_system_time_before_epoch() {
            let before_epoch = match SystemTime::UNIX_EPOCH.checked_sub(Duration::from_nanos(1_000))
            {
                Some(st) => st,
                None => return,
            };

            Timestamp::try_from(before_epoch)
                .expect_err("Timestamp should not be created from before epoch");
        }

        #[test]
        fn from_system_time_max() {
            let ts = Timestamp::from_unix_time(u64::MAX, 999_999_999, 0, 0);

            // Just make sure we don't panic
            let _: SystemTime = ts.into();
        }
    }
}
