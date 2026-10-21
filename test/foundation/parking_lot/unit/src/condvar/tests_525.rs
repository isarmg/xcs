#[cfg(test)]
mod tests {
    use crate::{Condvar, Mutex, MutexGuard};
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::thread;
    use std::thread::sleep;
    use std::thread::JoinHandle;
    use std::time::Duration;
    use std::time::Instant;

    #[test]
    fn smoke() {
        let c = Condvar::new();
        c.notify_one();
        c.notify_all();
    }

    #[test]
    fn notify_one() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let _t = thread::spawn(move || {
            let _g = m2.lock();
            c2.notify_one();
        });
        c.wait(&mut g);
    }

    #[test]
    fn notify_all() {
        const N: usize = 10;

        let data = Arc::new((Mutex::new(0), Condvar::new()));
        let (tx, rx) = channel();
        for _ in 0..N {
            let data = data.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (lock, cond) = &*data;
                let mut cnt = lock.lock();
                *cnt += 1;
                if *cnt == N {
                    tx.send(()).unwrap();
                }
                while *cnt != 0 {
                    cond.wait(&mut cnt);
                }
                tx.send(()).unwrap();
            });
        }
        drop(tx);

        let (lock, cond) = &*data;
        rx.recv().unwrap();
        let mut cnt = lock.lock();
        *cnt = 0;
        cond.notify_all();
        drop(cnt);

        for _ in 0..N {
            rx.recv().unwrap();
        }
    }

    #[test]
    fn notify_one_return_true() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let _t = thread::spawn(move || {
            let _g = m2.lock();
            assert!(c2.notify_one());
        });
        c.wait(&mut g);
    }

    #[test]
    fn notify_one_return_false() {
        let m = Arc::new(Mutex::new(()));
        let c = Arc::new(Condvar::new());

        let _t = thread::spawn(move || {
            let _g = m.lock();
            assert!(!c.notify_one());
        });
    }

    #[test]
    fn notify_all_return() {
        const N: usize = 10;

        let data = Arc::new((Mutex::new(0), Condvar::new()));
        let (tx, rx) = channel();
        for _ in 0..N {
            let data = data.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let (lock, cond) = &*data;
                let mut cnt = lock.lock();
                *cnt += 1;
                if *cnt == N {
                    tx.send(()).unwrap();
                }
                while *cnt != 0 {
                    cond.wait(&mut cnt);
                }
                tx.send(()).unwrap();
            });
        }
        drop(tx);

        let (lock, cond) = &*data;
        rx.recv().unwrap();
        let mut cnt = lock.lock();
        *cnt = 0;
        assert_eq!(cond.notify_all(), N);
        drop(cnt);

        for _ in 0..N {
            rx.recv().unwrap();
        }

        assert_eq!(cond.notify_all(), 0);
    }

    #[test]
    fn wait_for() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let no_timeout = c.wait_for(&mut g, Duration::from_millis(1));
        assert!(no_timeout.timed_out());

        let _t = thread::spawn(move || {
            let _g = m2.lock();
            c2.notify_one();
        });
        let timeout_res = c.wait_for(&mut g, Duration::from_secs(u64::max_value()));
        assert!(!timeout_res.timed_out());

        drop(g);
    }

    #[test]
    fn wait_until() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let no_timeout = c.wait_until(&mut g, Instant::now() + Duration::from_millis(1));
        assert!(no_timeout.timed_out());
        let _t = thread::spawn(move || {
            let _g = m2.lock();
            c2.notify_one();
        });
        let timeout_res = c.wait_until(
            &mut g,
            Instant::now() + Duration::from_millis(u32::max_value() as u64),
        );
        assert!(!timeout_res.timed_out());
        drop(g);
    }

    fn spawn_wait_while_notifier(
        mutex: Arc<Mutex<u32>>,
        cv: Arc<Condvar>,
        num_iters: u32,
        timeout: Option<Instant>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            for epoch in 1..=num_iters {
                // spin to wait for main test thread to block
                // before notifying it to wake back up and check
                // its condition.
                let mut sleep_backoff = Duration::from_millis(1);
                let _mutex_guard = loop {
                    let mutex_guard = mutex.lock();

                    if let Some(timeout) = timeout {
                        if Instant::now() >= timeout {
                            return;
                        }
                    }

                    if *mutex_guard == epoch {
                        break mutex_guard;
                    }

                    drop(mutex_guard);

                    // give main test thread a good chance to
                    // acquire the lock before this thread does.
                    sleep(sleep_backoff);
                    sleep_backoff *= 2;
                };

                cv.notify_one();
            }
        })
    }

    #[test]
    fn wait_while_until_internal_does_not_wait_if_initially_false() {
        let mutex = Arc::new(Mutex::new(0));
        let cv = Arc::new(Condvar::new());

        let condition = |counter: &mut u32| {
            *counter += 1;
            false
        };

        let mut mutex_guard = mutex.lock();
        let timeout_result = cv.wait_while_until_internal(&mut mutex_guard, condition, None);

        assert!(!timeout_result.timed_out());
        assert!(*mutex_guard == 1);
    }

    #[test]
    fn wait_while_until_internal_times_out_before_false() {
        let mutex = Arc::new(Mutex::new(0));
        let cv = Arc::new(Condvar::new());

        let num_iters = 3;
        let condition = |counter: &mut u32| {
            *counter += 1;
            true
        };

        let mut mutex_guard = mutex.lock();
        let timeout = Some(Instant::now() + Duration::from_millis(500));
        let handle = spawn_wait_while_notifier(mutex.clone(), cv.clone(), num_iters, timeout);

        let timeout_result = cv.wait_while_until_internal(&mut mutex_guard, condition, timeout);

        assert!(timeout_result.timed_out());
        assert!(*mutex_guard == num_iters + 1);

        // prevent deadlock with notifier
        drop(mutex_guard);
        handle.join().unwrap();
    }

    #[test]
    fn wait_while_until_internal() {
        let mutex = Arc::new(Mutex::new(0));
        let cv = Arc::new(Condvar::new());

        let num_iters = 4;

        let condition = |counter: &mut u32| {
            *counter += 1;
            *counter <= num_iters
        };

        let mut mutex_guard = mutex.lock();
        let handle = spawn_wait_while_notifier(mutex.clone(), cv.clone(), num_iters, None);

        let timeout_result = cv.wait_while_until_internal(&mut mutex_guard, condition, None);

        assert!(!timeout_result.timed_out());
        assert!(*mutex_guard == num_iters + 1);

        let timeout_result = cv.wait_while_until_internal(&mut mutex_guard, condition, None);
        handle.join().unwrap();

        assert!(!timeout_result.timed_out());
        assert!(*mutex_guard == num_iters + 2);
    }

    #[test]
    #[should_panic]
    fn two_mutexes() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let m3 = Arc::new(Mutex::new(()));
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        // Make sure we don't leave the child thread dangling
        struct PanicGuard<'a>(&'a Condvar);
        impl<'a> Drop for PanicGuard<'a> {
            fn drop(&mut self) {
                self.0.notify_one();
            }
        }

        let (tx, rx) = channel();
        let g = m.lock();
        let _t = thread::spawn(move || {
            let mut g = m2.lock();
            tx.send(()).unwrap();
            c2.wait(&mut g);
        });
        drop(g);
        rx.recv().unwrap();
        let _g = m.lock();
        let _guard = PanicGuard(&c);
        c.wait(&mut m3.lock());
    }

    #[test]
    fn two_mutexes_disjoint() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let m3 = Arc::new(Mutex::new(()));
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();

        let mut g = m.lock();
        let _t = thread::spawn(move || {
            let _g = m2.lock();
            c2.notify_one();
        });
        c.wait(&mut g);
        drop(g);

        let _ = c.wait_for(&mut m3.lock(), Duration::from_millis(1));
    }

    #[test]
    fn test_debug_condvar() {
        let c = Condvar::new();
        assert_eq!(format!("{:?}", c), "Condvar { .. }");
    }

    #[test]
    fn test_condvar_requeue() {
        let m = Arc::new(Mutex::new(()));
        let m2 = m.clone();
        let c = Arc::new(Condvar::new());
        let c2 = c.clone();
        let t = thread::spawn(move || {
            let mut g = m2.lock();
            c2.wait(&mut g);
        });

        let mut g = m.lock();
        while !c.notify_one() {
            // Wait for the thread to get into wait()
            MutexGuard::bump(&mut g);
            // Yield, so the other thread gets a chance to do something.
            // (At least Miri needs this, because it doesn't preempt threads.)
            thread::yield_now();
        }
        // The thread should have been requeued to the mutex, which we wake up now.
        drop(g);
        t.join().unwrap();
    }

    #[test]
    fn test_issue_129() {
        let locks = Arc::new((Mutex::new(()), Condvar::new()));

        let (tx, rx) = channel();
        for _ in 0..4 {
            let locks = locks.clone();
            let tx = tx.clone();
            thread::spawn(move || {
                let mut guard = locks.0.lock();
                locks.1.wait(&mut guard);
                locks.1.wait_for(&mut guard, Duration::from_millis(1));
                locks.1.notify_one();
                tx.send(()).unwrap();
            });
        }

        thread::sleep(Duration::from_millis(100));
        locks.1.notify_one();

        for _ in 0..4 {
            assert_eq!(rx.recv_timeout(Duration::from_millis(500)), Ok(()));
        }
    }
}
