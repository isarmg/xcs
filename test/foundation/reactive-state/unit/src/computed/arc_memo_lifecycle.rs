#[cfg(test)]
mod lifecycle_tests {
    use super::ArcMemo;
    use crate::{
        computed::Memo,
        graph::{ReactiveNode, ToAnySubscriber},
        owner::Owner,
        signal::RwSignal,
        traits::{Get, GetUntracked, Set},
    };

    fn subscribers(source: &ArcMemo<usize>) -> usize {
        source.inner.reactivity.read().unwrap().subscribers.len()
    }

    fn consumer(source: &ArcMemo<usize>) -> Owner {
        let owner = Owner::new();
        owner.with(|| {
            let source = source.clone();
            let memo = Memo::new(move |_| source.get());
            let _ = memo.get_untracked();
        });
        owner
    }

    #[test]
    fn disposed_consumers_do_not_accumulate_in_a_live_memo_source() {
        let app = Owner::new();
        let state = app.with(|| RwSignal::new(0usize));
        let source = app.with(|| ArcMemo::new(move |_| state.get()));
        for n in 0..128 {
            let page = Owner::new();
            let memo = page.with(|| {
                let source = source.clone();
                Memo::new(move |_| source.get())
            });
            assert_eq!(memo.get_untracked(), n);
            let subscriber = ArcMemo::from(memo).to_any_subscriber();
            page.cleanup();
            assert!(subscriber.1.upgrade().is_none());
            drop(subscriber);
            drop(page);
            state.set(n + 1);
            assert_eq!(subscribers(&source), 0);
            assert_eq!(source.get_untracked(), n + 1);
        }
    }

    #[test]
    fn memo_check_and_value_notifications_also_prune_dead_consumers() {
        let app = Owner::new();
        let state = app.with(|| RwSignal::new(0usize));
        let source = app.with(|| ArcMemo::new(move |_| state.get()));

        drop(consumer(&source));
        source.mark_check();
        assert_eq!(subscribers(&source), 0);

        drop(consumer(&source));
        source.mark_subscribers_check();
        assert_eq!(subscribers(&source), 0);

        // The consumer dies after invalidation but before lazy recomputation.
        let page = consumer(&source);
        state.set(1);
        drop(page);
        assert_eq!(source.get_untracked(), 1);
        assert_eq!(subscribers(&source), 0);
    }

    #[test]
    fn live_effect_can_recompute_a_memo_during_notification() {
        use crate::effect::ImmediateEffect;
        use std::sync::{mpsc, Arc, Mutex};
        use std::{thread, time::Duration};

        let (finished, completion) = mpsc::channel();
        thread::spawn(move || {
            let app = Owner::new();
            app.with(|| {
                let state = RwSignal::new(0usize);
                let source = ArcMemo::new(move |_| state.get() % 2);
                let values = Arc::new(Mutex::new(Vec::new()));
                let _effect = ImmediateEffect::new_isomorphic({
                    let source = source.clone();
                    let values = Arc::clone(&values);
                    move || values.lock().unwrap().push(source.get())
                });
                drop(consumer(&source));
                state.set(1);
                state.set(3); // Equal memo value must not rerun the effect.
                state.set(4);
                assert_eq!(*values.lock().unwrap(), [0, 1, 0]);
                assert_eq!(subscribers(&source), 1);
            });
            finished.send(()).unwrap();
        });
        completion
            .recv_timeout(Duration::from_secs(5))
            .expect("memo notification must release its lock before synchronous recomputation");
    }
}
