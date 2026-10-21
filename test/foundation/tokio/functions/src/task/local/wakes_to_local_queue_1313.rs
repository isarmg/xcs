#[test]
    fn wakes_to_local_queue() {
        use super::*;
        use crate::sync::Notify;
        let rt = crate::runtime::Builder::new_current_thread()
            .build()
            .expect("rt");
        rt.block_on(async {
            let local = LocalSet::new();
            let notify = Arc::new(Notify::new());
            let task = local.spawn_local({
                let notify = notify.clone();
                async move {
                    notify.notified().await;
                }
            });
            let mut run_until = Box::pin(local.run_until(async move {
                task.await.unwrap();
            }));

            // poll the run until future once
            std::future::poll_fn(|cx| {
                let _ = run_until.as_mut().poll(cx);
                Poll::Ready(())
            })
            .await;

            notify.notify_one();
            let task = unsafe { local.context.shared.local_state.task_pop_front() };
            // TODO(eliza): it would be nice to be able to assert that this is
            // the local task.
            assert!(
                task.is_some(),
                "task should have been notified to the LocalSet's local queue"
            );
        })
    }
