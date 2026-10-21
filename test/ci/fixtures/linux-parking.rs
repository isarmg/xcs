
#[cfg(test)]
mod tests {
    use synchronization::parking::{park, unpark_one, ParkResult, ParkToken, UnparkToken};
    use std::time::{Duration, Instant};
    #[test]
    fn timeout_and_validation() {
        let owned = Box::new(0u8);
        let key = (&*owned) as *const u8 as usize;
        unsafe {
            assert_eq!(
                park(
                    key,
                    || false,
                    || {},
                    |_, _| panic!("invalid must not timeout"),
                    ParkToken(1),
                    None
                ),
                ParkResult::Invalid
            );
            let mut called = false;
            let started = Instant::now();
            assert_eq!(
                park(
                    key,
                    || true,
                    || {},
                    |k, last| {
                        assert_eq!(k, key);
                        assert!(last);
                        called = true;
                    },
                    ParkToken(2),
                    Some(started + Duration::from_millis(10))
                ),
                ParkResult::TimedOut
            );
            assert!(called);
            assert!(started.elapsed() >= Duration::from_millis(10));
            assert_eq!(unpark_one(key, |_| UnparkToken(0)).unparked_threads, 0);
        }
    }
    #[test]
    fn wake_before_sleep_delivers_token() {
        let owned = Box::new(0u8);
        let key = (&*owned) as *const u8 as usize;
        unsafe {
            assert_eq!(
                park(
                    key,
                    || true,
                    || {
                        assert_eq!(
                            unpark_one(key, |r| {
                                assert_eq!(r.unparked_threads, 1);
                                UnparkToken(7)
                            })
                            .unparked_threads,
                            1
                        );
                    },
                    |_, _| panic!("unexpected timeout"),
                    ParkToken(3),
                    Some(Instant::now() + Duration::from_secs(2))
                ),
                ParkResult::Unparked(UnparkToken(7))
            );
        }
    }
}
