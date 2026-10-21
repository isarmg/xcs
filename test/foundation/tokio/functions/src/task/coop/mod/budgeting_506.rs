#[test]
    fn budgeting() {
        use std::future::poll_fn;
        use tokio_test::*;

        assert!(get().0.is_none());

        let coop = assert_ready!(task::spawn(()).enter(|cx, _| poll_proceed(cx)));

        assert!(get().0.is_none());
        drop(coop);
        assert!(get().0.is_none());

        budget(|| {
            assert_eq!(get().0, Budget::initial().0);

            let coop = assert_ready!(task::spawn(()).enter(|cx, _| poll_proceed(cx)));
            assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 1);
            drop(coop);
            // we didn't make progress
            assert_eq!(get().0, Budget::initial().0);

            let coop = assert_ready!(task::spawn(()).enter(|cx, _| poll_proceed(cx)));
            assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 1);
            coop.made_progress();
            drop(coop);
            // we _did_ make progress
            assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 1);

            let coop = assert_ready!(task::spawn(()).enter(|cx, _| poll_proceed(cx)));
            assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 2);
            coop.made_progress();
            drop(coop);
            assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 2);

            budget(|| {
                assert_eq!(get().0, Budget::initial().0);

                let coop = assert_ready!(task::spawn(()).enter(|cx, _| poll_proceed(cx)));
                assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 1);
                coop.made_progress();
                drop(coop);
                assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 1);
            });

            assert_eq!(get().0.unwrap(), Budget::initial().0.unwrap() - 2);
        });

        assert!(get().0.is_none());

        budget(|| {
            let n = get().0.unwrap();

            for _ in 0..n {
                let coop = assert_ready!(task::spawn(()).enter(|cx, _| poll_proceed(cx)));
                coop.made_progress();
            }

            let mut task = task::spawn(poll_fn(|cx| {
                let coop = std::task::ready!(poll_proceed(cx));
                coop.made_progress();
                Poll::Ready(())
            }));

            assert_pending!(task.poll());
        });
    }
