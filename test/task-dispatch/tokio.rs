use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Barrier};
use task_dispatch::Executor;

#[test]
fn initialized_backend_works_in_each_threads_runtime() {
    Executor::init_tokio().unwrap();
    assert!(Executor::init_tokio().is_err());
    let barrier = Arc::new(Barrier::new(3));
    let threads: Vec<_> = (0..3)
        .map(|_| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();
                let local = tokio::task::LocalSet::new();
                barrier.wait();
                local.block_on(&runtime, async {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    Executor::spawn(async move { tx.send(42).unwrap() });
                    assert_eq!(rx.await.unwrap(), 42);
                    let value = Rc::new(Cell::new(0));
                    let task_value = value.clone();
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    Executor::spawn_local(async move {
                        task_value.set(7);
                        tx.send(()).unwrap();
                    });
                    rx.await.unwrap();
                    assert_eq!(value.get(), 7);
                    Executor::tick().await;
                });
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
