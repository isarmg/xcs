#[test]
    fn smoke() {
        let rt = rt();
        rt.block_on(async move {
            let registry = Registry::new(vec![
                EventInfo::default(),
                EventInfo::default(),
                EventInfo::default(),
            ]);

            let first = registry.register_listener(0);
            let second = registry.register_listener(1);
            let third = registry.register_listener(2);

            let (fire, wait) = oneshot::channel();

            crate::spawn(async {
                wait.await.expect("wait failed");

                // Record some events which should get coalesced
                registry.record_event(0);
                registry.record_event(0);
                registry.record_event(1);
                registry.record_event(1);
                registry.broadcast();

                // Yield so the previous broadcast can get received
                //
                // This yields many times since the block_on task is only polled every 61
                // ticks.
                for _ in 0..100 {
                    crate::task::yield_now().await;
                }

                // Send subsequent signal
                registry.record_event(0);
                registry.broadcast();

                drop(registry);
            });

            let _ = fire.send(());
            let all = future::join3(collect(first), collect(second), collect(third));

            let (first_results, second_results, third_results) = all.await;
            assert_eq!(2, first_results.len());
            assert_eq!(1, second_results.len());
            assert_eq!(0, third_results.len());
        });
    }
