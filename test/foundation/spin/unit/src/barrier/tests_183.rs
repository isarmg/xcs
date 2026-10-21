#[cfg(test)]
mod tests {
    use std::prelude::v1::*;

    use std::sync::mpsc::{channel, TryRecvError};
    use std::sync::Arc;
    use std::thread;

    type Barrier = super::Barrier;

    fn use_barrier(n: usize, barrier: Arc<Barrier>) {
        let (tx, rx) = channel();

        let mut ts = Vec::new();
        for _ in 0..n - 1 {
            let c = barrier.clone();
            let tx = tx.clone();
            ts.push(thread::spawn(move || {
                tx.send(c.wait().is_leader()).unwrap();
            }));
        }

        // At this point, all spawned threads should be blocked,
        // so we shouldn't get anything from the port
        assert!(match rx.try_recv() {
            Err(TryRecvError::Empty) => true,
            _ => false,
        });

        let mut leader_found = barrier.wait().is_leader();

        // Now, the barrier is cleared and we should get data.
        for _ in 0..n - 1 {
            if rx.recv().unwrap() {
                assert!(!leader_found);
                leader_found = true;
            }
        }
        assert!(leader_found);

        for t in ts {
            t.join().unwrap();
        }
    }

    #[test]
    fn test_barrier() {
        const N: usize = 10;

        let barrier = Arc::new(Barrier::new(N));

        use_barrier(N, barrier.clone());

        // use barrier twice to ensure it is reusable
        use_barrier(N, barrier.clone());
    }
}
