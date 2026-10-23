use std::cell::Cell;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Barrier};
use std::time::Duration;
use task_dispatch::Executor;

#[test]
fn concurrent_initialization_and_thread_local_progress() {
    let barrier = Arc::new(Barrier::new(4));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let initialized = Executor::init_futures_executor().is_ok();
                barrier.wait();
                let value = Rc::new(Cell::new(0));
                let task_value = value.clone();
                Executor::spawn_local(async move {
                    task_value.set(1);
                    // Re-entrant polling must not borrow the local pool twice.
                    Executor::poll_local();
                    let nested_value = task_value.clone();
                    Executor::spawn_local(async move { nested_value.set(2) });
                });
                Executor::poll_local();
                assert_eq!(value.get(), 2);
                let (tx, rx) = mpsc::channel();
                Executor::spawn(async move { tx.send(42).unwrap() });
                assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), 42);
                initialized
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|thread| thread.join().unwrap().then_some(()))
            .count(),
        1
    );
    assert!(Executor::init_futures_executor().is_err());
}
