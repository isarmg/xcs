#[cfg(test)]
mod tests {
    use std::thread;
    use tokio_sync::oneshot;
    use super::*;

    fn block_on<F: Future>(f: F) -> F::Output {
        tokio_executor::enter()
            .expect("block_on enter")
            .block_on(f)
    }

    #[test]
    fn want_ready() {
        let (mut gv, mut tk) = new();

        tk.want();

        block_on(gv.want()).unwrap();
    }

    #[test]
    fn want_notify_0() {
        let (mut gv, mut tk) = new();
        let (tx, rx) = oneshot::channel();

        thread::spawn(move || {
            tk.want();
            // use a oneshot to keep this thread alive
            // until other thread was notified of want
            block_on(rx).expect("rx");
        });

        block_on(gv.want()).expect("want");

        assert!(gv.is_wanting(), "still wanting after poll_want success");
        assert!(gv.give(), "give is true when wanting");

        assert!(!gv.is_wanting(), "no longer wanting after give");
        assert!(!gv.is_canceled(), "give doesn't cancel");

        assert!(!gv.give(), "give is false if not wanting");

        tx.send(()).expect("tx");
    }

    /*
    /// This tests that if the Giver moves tasks after parking,
    /// it will still wake up the correct task.
    #[test]
    fn want_notify_moving_tasks() {
        use std::sync::Arc;
        use futures::executor::{spawn, Notify, NotifyHandle};

        struct WantNotify;

        impl Notify for WantNotify {
            fn notify(&self, _id: usize) {
            }
        }

        fn n() -> NotifyHandle {
            Arc::new(WantNotify).into()
        }

        let (mut gv, mut tk) = new();

        let mut s = spawn(poll_fn(move || {
            gv.poll_want()
        }));

        // Register with t1 as the task::current()
        let t1 = n();
        assert!(s.poll_future_notify(&t1, 1).unwrap().is_not_ready());

        thread::spawn(move || {
            thread::sleep(::std::time::Duration::from_millis(100));
            tk.want();
        });

        // And now, move to a ThreadNotify task.
        s.into_inner().wait().expect("poll_want");
    }
    */

    #[test]
    fn cancel() {
        // explicit
        let (mut gv, mut tk) = new();

        assert!(!gv.is_canceled());

        tk.cancel();

        assert!(gv.is_canceled());
        block_on(gv.want()).unwrap_err();

        // implicit
        let (mut gv, tk) = new();

        assert!(!gv.is_canceled());

        drop(tk);

        assert!(gv.is_canceled());
        block_on(gv.want()).unwrap_err();

        // notifies
        let (mut gv, tk) = new();

        thread::spawn(move || {
            let _tk = tk;
            // and dropped
        });

        block_on(gv.want()).unwrap_err();
    }

    #[test]
    fn unwant() {
        let (gv, mut tk) = new();

        tk.want();
        assert!(gv.is_wanting());

        tk.unwant();
        assert!(!gv.is_wanting());
        assert!(!gv.give());

        // Only a want is withdrawn: a canceled Taker stays canceled.
        tk.cancel();
        tk.unwant();
        assert!(gv.is_canceled());
    }

    /*
    #[test]
    fn stress() {
        let nthreads = 5;
        let nwants = 100;

        for _ in 0..nthreads {
            let (mut gv, mut tk) = new();
            let (mut tx, mut rx) = mpsc::channel(0);

            // rx thread
            thread::spawn(move || {
                let mut cnt = 0;
                poll_fn(move || {
                    while cnt < nwants {
                        let n = match rx.poll().expect("rx poll") {
                            Async::Ready(n) => n.expect("rx opt"),
                            Async::NotReady => {
                                tk.want();
                                return Ok(Async::NotReady);
                            },
                        };
                        assert_eq!(cnt, n);
                        cnt += 1;
                    }
                    Ok::<_, ()>(Async::Ready(()))
                }).wait().expect("rx wait");
            });

            // tx thread
            thread::spawn(move || {
                let mut cnt = 0;
                let nsent = poll_fn(move || {
                    loop {
                        while let Ok(()) = tx.try_send(cnt) {
                            cnt += 1;
                        }
                        match gv.poll_want() {
                            Ok(Async::Ready(_)) => (),
                            Ok(Async::NotReady) => return Ok::<_, ()>(Async::NotReady),
                            Err(_) => return Ok(Async::Ready(cnt)),
                        }
                    }
                }).wait().expect("tx wait");

                assert_eq!(nsent, nwants);
            }).join().expect("thread join");
        }
    }
    */
}
