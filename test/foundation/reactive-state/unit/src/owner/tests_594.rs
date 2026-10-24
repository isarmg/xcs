include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/reactive-state/unit/src/owner/child_registry.rs"
));

#[cfg(test)]
mod tests {
    use crate::{
        owner::{on_cleanup, Owner, StoredValue},
        signal::signal,
        traits::{GetValue, Set},
    };
    use std::{sync::mpsc, thread, time::Duration};

    fn does_not_deadlock(f: impl FnOnce() + Send + 'static) -> bool {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            f();
            _ = tx.send(());
        });
        rx.recv_timeout(Duration::from_secs(5)).is_ok()
    }

    fn dispose_through_arena(root: &Owner, setup: impl FnOnce(), dispose: impl FnOnce(Owner)) {
        let (inner, outer) = root.with(|| (Owner::new(), Owner::new()));

        inner.with(setup);
        outer.with(|| {
            _ = StoredValue::new(inner);
        });

        dispose(outer);
    }

    #[test]
    fn dropping_owner_held_in_arena_removes_its_nodes() {
        assert!(does_not_deadlock(|| {
            let root = Owner::new();
            dispose_through_arena(
                &root,
                || {
                    _ = signal(0);
                },
                drop,
            );
        }));
    }

    #[test]
    fn dropping_owner_held_in_arena_runs_cleanups_accessing_arena() {
        assert!(does_not_deadlock(|| {
            let root = Owner::new();
            let value = root.with(|| StoredValue::new(0));

            dispose_through_arena(
                &root,
                move || {
                    let (_, set_count) = signal(0);
                    on_cleanup(move || {
                        _ = value.get_value();
                        set_count.set(1);
                    });
                },
                drop,
            );
        }));
    }

    #[test]
    fn cleaning_up_owner_holding_owner_in_arena() {
        assert!(does_not_deadlock(|| {
            let root = Owner::new();
            dispose_through_arena(
                &root,
                || {
                    _ = signal(0);
                },
                |outer| outer.cleanup(),
            );
        }));
    }
}

// Exercise page cleanup against a long-lived application signal without an executor.
#[cfg(test)]
mod lifetime_registry_regressions {
    use crate::{
        computed::{ArcMemo, Memo},
        graph::ToAnySubscriber,
        owner::Owner,
        signal::{ArcRwSignal, RwSignal},
        traits::{Get, GetUntracked, Set},
    };
    use std::collections::HashSet;

    #[test]
    fn disposed_page_memos_do_not_accumulate_in_a_live_app_signal() {
        let app = Owner::new();
        let state = app.with(|| RwSignal::new(0usize));
        let source = ArcRwSignal::from(state);
        let mut identities = HashSet::new();
        let mut sizes = Vec::new();
        for n in 0..128 {
            let page = app.with(Owner::new);
            let memo = page.with(|| Memo::new(move |_| state.get()));
            assert_eq!(memo.get_untracked(), n);
            let subscriber = ArcMemo::from(memo).to_any_subscriber();
            identities.insert(subscriber.0);
            page.cleanup();
            assert!(subscriber.1.upgrade().is_none(), "memo remains live");
            drop(subscriber); // Do not retain audit-owned Weak handles across iterations.
            drop(page);
            state.set(n + 1); // The real RwSignal notify path must also be exercised.
            assert_eq!(state.get_untracked(), n + 1);
            sizes.push(source.inner.read().unwrap().len());
        }
        let live = source
            .inner
            .read()
            .unwrap()
            .clone()
            .into_iter()
            .filter(|subscriber| subscriber.1.upgrade().is_some())
            .count();
        assert_eq!(live, 0, "this is not a delayed task or live memo");
        // Dead dependencies must not accumulate across page lifetimes.
        assert!(
            sizes.iter().all(|size| *size == 0),
            "dead subscriptions retained: sizes={sizes:?}, unique_ids={}",
            identities.len()
        );
    }

    #[test]
    fn notification_pruning_preserves_live_subscription_order() {
        use crate::effect::ImmediateEffect;
        use std::sync::{mpsc, Arc, Mutex};
        use std::{thread, time::Duration};

        let (finished, completion) = mpsc::channel();
        thread::spawn(move || {
            let app = Owner::new();
            app.with(|| {
                let state = RwSignal::new(0);
                let source = ArcRwSignal::from(state);
                let calls = Arc::new(Mutex::new(Vec::new()));
                let subscribe = |id| {
                    let calls = Arc::clone(&calls);
                    ImmediateEffect::new_isomorphic(move || {
                        let value = state.get();
                        calls.lock().unwrap().push((id, value));
                    })
                };
                let _first = subscribe(1);
                let removed = subscribe(2);
                let _last = subscribe(3);
                calls.lock().unwrap().clear();
                drop(removed);

                // Live effects synchronously re-enter the source to clear and
                // renew their subscriptions while this snapshot is notified.
                state.set(1);
                state.set(2);
                assert_eq!(*calls.lock().unwrap(), [(1, 1), (3, 1), (1, 2), (3, 2)]);
                assert_eq!(source.inner.read().unwrap().len(), 2);
            });
            finished.send(()).unwrap();
        });
        completion
            .recv_timeout(Duration::from_secs(5))
            .expect("notification must release the source lock before running live effects");
    }
}
