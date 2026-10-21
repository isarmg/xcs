#[cfg(test)]
        mod tests {
            use core::time::Duration;

            use super::*;

            use crate::{Timestamp, Uuid};

            #[test]
            fn context() {
                let seconds = 1_496_854_535;
                let subsec_nanos = 812_946_000;

                let context = ContextV7::new();

                let ts1 = Timestamp::from_unix(&context, seconds, subsec_nanos);
                assert_eq!(42, ts1.usable_counter_bits);

                // Backwards second
                let seconds = 1_496_854_534;

                let ts2 = Timestamp::from_unix(&context, seconds, subsec_nanos);

                // The backwards time should be ignored
                // The counter should still increment
                assert_eq!(ts1.seconds, ts2.seconds);
                assert_eq!(ts1.subsec_nanos, ts2.subsec_nanos);
                assert_eq!(ts1.counter + 1, ts2.counter);

                // Forwards second
                let seconds = 1_496_854_536;

                let ts3 = Timestamp::from_unix(&context, seconds, subsec_nanos);

                // The counter should have reseeded
                assert_ne!(ts2.counter + 1, ts3.counter);
                assert_ne!(0, ts3.counter);
            }

            #[test]
            fn context_wrap() {
                let seconds = 1_496_854_535u64;
                let subsec_nanos = 812_946_000u32;

                // This context will wrap
                let context = ContextV7 {
                    timestamp: Cell::new(ReseedingTimestamp::from_ts(seconds, subsec_nanos)),
                    adjust: Adjust::by_millis(0),
                    precision: Precision {
                        bits: 0,
                        mask: 0,
                        factor: 0,
                        shift: 0,
                    },
                    counter: Cell::new(Counter {
                        value: u64::MAX >> 22,
                    }),
                };

                let ts = Timestamp::from_unix(&context, seconds, subsec_nanos);

                // The timestamp should be incremented by 1ms
                let expected_ts = Duration::new(seconds, subsec_nanos) + Duration::from_millis(1);
                assert_eq!(expected_ts.as_secs(), ts.seconds);
                assert_eq!(expected_ts.subsec_nanos(), ts.subsec_nanos);

                // The counter should have reseeded
                assert!(ts.counter < (u64::MAX >> 22) as u128);
                assert_ne!(0, ts.counter);
            }

            #[test]
            fn context_shift() {
                let seconds = 1_496_854_535;
                let subsec_nanos = 812_946_000;

                let context = ContextV7::new().with_adjust_by_millis(1);

                let ts = Timestamp::from_unix(&context, seconds, subsec_nanos);

                assert_eq!((1_496_854_535, 813_946_000), ts.to_unix());
            }

            #[test]
            fn context_additional_precision() {
                let seconds = 1_496_854_535;
                let subsec_nanos = 812_946_000;

                let context = ContextV7::new().with_additional_precision();

                let ts1 = Timestamp::from_unix(&context, seconds, subsec_nanos);

                // NOTE: Future changes in rounding may change this value slightly
                assert_eq!(3861, ts1.counter >> 30);

                assert!(ts1.counter < (u64::MAX >> 22) as u128);

                // Generate another timestamp; it should continue to sort
                let ts2 = Timestamp::from_unix(&context, seconds, subsec_nanos);

                assert!(Uuid::new_v7(ts2) > Uuid::new_v7(ts1));

                // Generate another timestamp with an extra nanosecond
                let subsec_nanos = subsec_nanos + 1;

                let ts3 = Timestamp::from_unix(&context, seconds, subsec_nanos);

                assert!(Uuid::new_v7(ts3) > Uuid::new_v7(ts2));
            }

            #[test]
            fn context_additional_precision_bits() {
                let seconds = 1_496_854_535;
                let subsec_nanos = 812_946_000;

                // 10 bits leaves the low 2 bits of `rand_a` for random data, which suits
                // platforms with microsecond precision
                let context = ContextV7::new().with_additional_precision_bits(10);

                let ts = Timestamp::from_unix(&context, seconds, subsec_nanos);

                // The submillisecond precision occupies the leftmost 10 of the 42 counter bits
                // NOTE: Future changes in rounding may change this value slightly
                assert_eq!(968, ts.counter >> 32);

                assert!(ts.counter < (u64::MAX >> 22) as u128);

                // The full method is just the 12-bit form of this one
                let full = Timestamp::from_unix(
                    &ContextV7::new().with_additional_precision(),
                    seconds,
                    subsec_nanos,
                );
                let bits12 = Timestamp::from_unix(
                    &ContextV7::new().with_additional_precision_bits(12),
                    seconds,
                    subsec_nanos,
                );
                assert_eq!(full.counter >> 30, bits12.counter >> 30);

                // Values above 12 are capped at 12
                let capped = Timestamp::from_unix(
                    &ContextV7::new().with_additional_precision_bits(64),
                    seconds,
                    subsec_nanos,
                );
                assert_eq!(bits12.counter >> 30, capped.counter >> 30);

                // Zero bits disables additional precision entirely
                let none = ContextV7::new().with_additional_precision_bits(0);
                assert_eq!(0, none.precision.bits);
            }

            #[test]
            fn context_overflow() {
                let seconds = u64::MAX;
                let subsec_nanos = u32::MAX;

                // Ensure we don't panic
                for context in [
                    ContextV7::new(),
                    ContextV7::new().with_additional_precision(),
                    ContextV7::new().with_adjust_by_millis(u32::MAX),
                ] {
                    Timestamp::from_unix(&context, seconds, subsec_nanos);
                }
            }

            #[test]
            fn context_subsec_ordering() {
                let context = ContextV7::new();

                // Wall-clock order of incoming timestamps is b, a, c
                // UUID order is a, b, c
                let a = Timestamp::from_unix(&context, 100, 0);
                let b = Timestamp::from_unix(&context, 99, 2_000_000);
                let c = Timestamp::from_unix(&context, 100, 1_000_000);

                assert!(Uuid::new_v7(a) < Uuid::new_v7(b), "a: {a:?} < b: {b:?}");
                assert!(Uuid::new_v7(b) < Uuid::new_v7(c), "b: {b:?} < c: {c:?}");
            }
        }
