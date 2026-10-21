#[cfg(test)]
        mod tests {
            use crate::Timestamp;

            use super::*;

            #[test]
            fn context() {
                let seconds = 1_496_854_535;
                let subsec_nanos = 812_946_000;

                let context = ContextV1::new(u16::MAX >> 2);

                let ts = Timestamp::from_unix(&context, seconds, subsec_nanos);
                assert_eq!(16383, ts.counter);
                assert_eq!(14, ts.usable_counter_bits);

                let seconds = 1_496_854_536;

                let ts = Timestamp::from_unix(&context, seconds, subsec_nanos);
                assert_eq!(0, ts.counter);

                let seconds = 1_496_854_535;

                let ts = Timestamp::from_unix(&context, seconds, subsec_nanos);
                assert_eq!(1, ts.counter);
            }

            #[test]
            fn context_overflow() {
                let seconds = u64::MAX;
                let subsec_nanos = u32::MAX;

                let context = ContextV1::new(u16::MAX);

                // Ensure we don't panic
                Timestamp::from_unix(&context, seconds, subsec_nanos);
            }
        }
