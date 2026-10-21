use concurrent_collections::{
    deque::{Injector, Steal, Worker},
    queue::{ArrayQueue, SegQueue},
};
use rayon::prelude::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
#[test]
fn rayon_parallel_consumer() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    assert_eq!(
        pool.install(|| (0u64..100_000).into_par_iter().map(|n| n * n).sum::<u64>()),
        (0u64..100_000).map(|n| n * n).sum::<u64>()
    );
}
#[test]
fn bounded_mpmc_exactly_once() {
    let queue = Arc::new(ArrayQueue::new(37));
    let seen: Arc<Vec<AtomicUsize>> = Arc::new((0..4000).map(|_| AtomicUsize::new(0)).collect());
    std::thread::scope(|s| {
        for producer in 0..4 {
            let queue = queue.clone();
            s.spawn(move || {
                for i in 0..1000 {
                    let mut value = producer * 1000 + i;
                    loop {
                        match queue.push(value) {
                            Ok(()) => break,
                            Err(v) => {
                                value = v;
                                std::thread::yield_now();
                            }
                        }
                    }
                }
            });
        }
        for _ in 0..4 {
            let queue = queue.clone();
            let seen = seen.clone();
            s.spawn(move || {
                for _ in 0..1000 {
                    loop {
                        if let Some(v) = queue.pop() {
                            seen[v].fetch_add(1, Ordering::SeqCst);
                            break;
                        }
                        std::thread::yield_now();
                    }
                }
            });
        }
    });
    assert!(seen.iter().all(|n| n.load(Ordering::SeqCst) == 1));
    assert!(queue.is_empty());
}
#[test]
fn unbounded_fifo_and_drop_exactly_once() {
    struct Item(Arc<AtomicUsize>);
    impl Drop for Item {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    {
        let queue = SegQueue::new();
        for _ in 0..1000 {
            queue.push(Item(count.clone()));
        }
        for _ in 0..500 {
            drop(queue.pop().unwrap());
        }
    }
    assert_eq!(count.load(Ordering::SeqCst), 1000);
    let q = SegQueue::new();
    for n in 0..1000 {
        q.push(n);
    }
    for n in 0..1000 {
        assert_eq!(q.pop(), Some(n));
    }
}
#[test]
fn batch_steal_preserves_all_tasks() {
    let q = Injector::new();
    for n in 0..1000 {
        q.push(n);
    }
    let w = Worker::new_fifo();
    let mut tasks = Vec::new();
    loop {
        match q.steal_batch_and_pop(&w) {
            Steal::Success(n) => tasks.push(n),
            Steal::Empty => break,
            Steal::Retry => continue,
        }
        while let Some(n) = w.pop() {
            tasks.push(n);
        }
    }
    tasks.sort();
    assert_eq!(tasks, (0..1000).collect::<Vec<_>>());
}
