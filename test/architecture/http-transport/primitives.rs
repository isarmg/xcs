//! Additional contracts for the transport-owned dispatch and wakeup primitives.
use crate::common::{demand, task_waker::AtomicWaker, try_lock::TryLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;

#[derive(Default)]
struct Count(AtomicUsize);
impl Wake for Count {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn counter() -> (Arc<Count>, Waker) {
    let count = Arc::new(Count::default());
    (count.clone(), Waker::from(count))
}

#[test]
fn demand_moves_between_tasks() {
    let (mut giver, mut taker) = demand::new();
    let (old_count, old) = counter();
    let (new_count, new) = counter();
    assert!(giver.poll_want(&mut Context::from_waker(&old)).is_pending());
    assert!(giver.poll_want(&mut Context::from_waker(&new)).is_pending());
    assert_eq!(old_count.0.load(Ordering::SeqCst), 1);
    taker.want();
    assert_eq!(new_count.0.load(Ordering::SeqCst), 1);
    assert!(matches!(
        giver.poll_want(&mut Context::from_waker(&new)),
        Poll::Ready(Ok(()))
    ));
    assert!(giver.give());
    assert!(!giver.give());
}

#[test]
fn demand_cancel_wakes_registered_giver_and_stays_closed() {
    let (mut giver, mut taker) = demand::new();
    let (count, waker) = counter();
    assert!(giver
        .poll_want(&mut Context::from_waker(&waker))
        .is_pending());
    taker.cancel();
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
    assert!(matches!(
        giver.poll_want(&mut Context::from_waker(&waker)),
        Poll::Ready(Err(_))
    ));
    assert!(!giver.give());
    taker.unwant();
    assert!(giver.is_canceled());
}

#[test]
fn demand_shared_givers_observe_receiver_drop() {
    let (giver, mut taker) = demand::new();
    let shared = giver.shared();
    let other = shared.clone();
    taker.want();
    assert!(shared.is_wanting());
    drop(taker);
    assert!(other.is_canceled());
    assert!(!shared.is_wanting());
}

#[test]
fn demand_receiver_can_outlive_sender() {
    let (giver, mut taker) = demand::new();
    drop(giver);
    taker.want();
    taker.unwant();
    taker.cancel();
}

#[test]
fn demand_registration_notification_race() {
    for _ in 0..128 {
        let (mut giver, mut taker) = demand::new();
        let barrier = Arc::new(Barrier::new(2));
        let worker_barrier = barrier.clone();
        let worker = thread::spawn(move || {
            worker_barrier.wait();
            taker.want();
            // Keep the receiver alive until the giver has observed readiness.
            taker
        });
        barrier.wait();
        futures::executor::block_on(giver.want()).unwrap();
        let taker = worker.join().unwrap();
        assert!(giver.give());
        drop(taker);
        assert!(giver.is_canceled());
    }
}

#[test]
fn try_lock_excludes_concurrent_mutation() {
    let lock = Arc::new(TryLock::new(0usize));
    let mut workers = Vec::new();
    for _ in 0..4 {
        let lock = lock.clone();
        workers.push(thread::spawn(move || {
            for _ in 0..1000 {
                loop {
                    if let Some(mut value) = lock.try_lock() {
                        *value += 1;
                        break;
                    }
                    thread::yield_now();
                }
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(Arc::try_unwrap(lock).unwrap().into_inner(), 4000);
}

#[test]
fn try_lock_rejects_weak_orderings() {
    for (lock, unlock) in [
        (Ordering::Relaxed, Ordering::Release),
        (Ordering::Acquire, Ordering::Relaxed),
    ] {
        let value = TryLock::new(1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            value.try_lock_explicit(lock, unlock)
        }));
        assert!(result.is_err());
        assert_eq!(*value.try_lock().unwrap(), 1);
    }
}

#[test]
fn try_lock_guard_releases_during_unwind() {
    let value = TryLock::new(1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut guard = value.try_lock().unwrap();
        *guard = 2;
        panic!("exercise guard drop");
    }));
    assert!(result.is_err());
    assert_eq!(*value.try_lock().unwrap(), 2);
}

#[test]
fn atomic_waker_uses_latest_task() {
    let slot = AtomicWaker::new();
    let (old_count, old) = counter();
    let (new_count, new) = counter();
    slot.register(&old);
    slot.register(&new);
    slot.wake();
    assert_eq!(old_count.0.load(Ordering::SeqCst), 0);
    assert_eq!(new_count.0.load(Ordering::SeqCst), 1);
    slot.wake();
    assert_eq!(new_count.0.load(Ordering::SeqCst), 1);
}

#[test]
fn atomic_waker_take_consumes_registration() {
    let slot = AtomicWaker::default();
    let (count, waker) = counter();
    slot.register(&waker);
    let registered = slot.take().unwrap();
    assert!(slot.take().is_none());
    slot.wake();
    assert_eq!(count.0.load(Ordering::SeqCst), 0);
    registered.wake();
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
}

#[test]
fn atomic_waker_early_wake_does_not_spuriously_notify_later_task() {
    let slot = AtomicWaker::new();
    slot.wake();
    let (count, waker) = counter();
    slot.register(&waker);
    assert_eq!(count.0.load(Ordering::SeqCst), 0);
    slot.wake();
    assert_eq!(count.0.load(Ordering::SeqCst), 1);
}

#[test]
fn atomic_waker_register_wake_race_has_no_lost_notification() {
    for _ in 0..128 {
        let slot = Arc::new(AtomicWaker::new());
        let ready = Arc::new(AtomicBool::new(false));
        let barrier = Arc::new(Barrier::new(2));
        let (finished, result) = std::sync::mpsc::channel();
        let worker = {
            let slot = slot.clone();
            let ready = ready.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                futures::executor::block_on(std::future::poll_fn(|cx| {
                    if ready.load(Ordering::Acquire) {
                        return Poll::Ready(());
                    }
                    slot.register(cx.waker());
                    // Required registration protocol: recheck after registering.
                    if ready.load(Ordering::Acquire) {
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                }));
                finished.send(()).unwrap();
            })
        };
        barrier.wait();
        ready.store(true, Ordering::Release);
        slot.wake();
        result
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("lost wakeup");
        worker.join().unwrap();
    }
}
