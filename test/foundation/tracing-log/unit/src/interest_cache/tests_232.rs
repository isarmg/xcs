#[cfg(test)]
mod tests {
    use super::*;

    fn lock_for_test() -> impl Drop {
        // We need to make sure only one test runs at a time.
        static LOCK: Lazy<Mutex<()>> = Lazy::new(Mutex::new);

        match LOCK.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        }
    }

    #[test]
    fn test_when_disabled_the_callback_is_always_called() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::disabled();

        std::thread::spawn(|| {
            let metadata = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy")
                .build();
            let mut count = 0;
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 2);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_when_enabled_the_callback_is_called_only_once_for_a_high_enough_verbosity() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let metadata = log::MetadataBuilder::new()
                .level(Level::Debug)
                .target("dummy")
                .build();
            let mut count = 0;
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_when_core_interest_cache_is_rebuilt_this_cache_is_also_flushed() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let metadata = log::MetadataBuilder::new()
                .level(Level::Debug)
                .target("dummy")
                .build();
            {
                let mut count = 0;
                try_cache(&metadata, || {
                    count += 1;
                    true
                });
                try_cache(&metadata, || {
                    count += 1;
                    true
                });
                assert_eq!(count, 1);
            }
            tracing_core::callsite::rebuild_interest_cache();
            {
                let mut count = 0;
                try_cache(&metadata, || {
                    count += 1;
                    true
                });
                try_cache(&metadata, || {
                    count += 1;
                    true
                });
                assert_eq!(count, 1);
            }
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_when_enabled_the_callback_is_always_called_for_a_low_enough_verbosity() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let metadata = log::MetadataBuilder::new()
                .level(Level::Info)
                .target("dummy")
                .build();
            let mut count = 0;
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 2);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_different_log_levels_are_cached_separately() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let metadata_debug = log::MetadataBuilder::new()
                .level(Level::Debug)
                .target("dummy")
                .build();
            let metadata_trace = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy")
                .build();
            let mut count_debug = 0;
            let mut count_trace = 0;
            try_cache(&metadata_debug, || {
                count_debug += 1;
                true
            });
            try_cache(&metadata_trace, || {
                count_trace += 1;
                true
            });
            try_cache(&metadata_debug, || {
                count_debug += 1;
                true
            });
            try_cache(&metadata_trace, || {
                count_trace += 1;
                true
            });
            assert_eq!(count_debug, 1);
            assert_eq!(count_trace, 1);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_different_log_targets_are_cached_separately() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let metadata_1 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy_1")
                .build();
            let metadata_2 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy_2")
                .build();
            let mut count_1 = 0;
            let mut count_2 = 0;
            try_cache(&metadata_1, || {
                count_1 += 1;
                true
            });
            try_cache(&metadata_2, || {
                count_2 += 1;
                true
            });
            try_cache(&metadata_1, || {
                count_1 += 1;
                true
            });
            try_cache(&metadata_2, || {
                count_2 += 1;
                true
            });
            assert_eq!(count_1, 1);
            assert_eq!(count_2, 1);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_when_cache_runs_out_of_space_the_callback_is_called_again() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default()
            .with_min_verbosity(Level::Debug)
            .with_lru_cache_size(1);

        std::thread::spawn(|| {
            let metadata_1 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy_1")
                .build();
            let metadata_2 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy_2")
                .build();
            let mut count = 0;
            try_cache(&metadata_1, || {
                count += 1;
                true
            });
            try_cache(&metadata_1, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
            try_cache(&metadata_2, || true);
            try_cache(&metadata_1, || {
                count += 1;
                true
            });
            assert_eq!(count, 2);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_cache_returns_previously_computed_value() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let metadata_1 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy_1")
                .build();
            let metadata_2 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target("dummy_2")
                .build();
            try_cache(&metadata_1, || true);
            assert_eq!(try_cache(&metadata_1, || { unreachable!() }), true);
            try_cache(&metadata_2, || false);
            assert_eq!(try_cache(&metadata_2, || { unreachable!() }), false);
        })
        .join()
        .unwrap();
    }

    #[test]
    fn test_cache_handles_non_static_target_string() {
        let _lock = lock_for_test();

        *CONFIG.lock().unwrap() = InterestCacheConfig::default().with_min_verbosity(Level::Debug);

        std::thread::spawn(|| {
            let mut target = *b"dummy_1";
            let metadata_1 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target(std::str::from_utf8(&target).unwrap())
                .build();

            try_cache(&metadata_1, || true);
            assert_eq!(try_cache(&metadata_1, || { unreachable!() }), true);

            *target.last_mut().unwrap() = b'2';
            let metadata_2 = log::MetadataBuilder::new()
                .level(Level::Trace)
                .target(std::str::from_utf8(&target).unwrap())
                .build();

            try_cache(&metadata_2, || false);
            assert_eq!(try_cache(&metadata_2, || { unreachable!() }), false);
        })
        .join()
        .unwrap();
    }
}
