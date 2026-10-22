//! Behavioral contracts for the shared SQLite/reactive-state notification substrate.
use async_notification::{
    event::{Event, Listener},
    Mutex, OnceCell, RwLock, Semaphore,
};
use std::{
    future::Future,
    pin::{pin, Pin},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn poll<F: Future>(future: Pin<&mut F>, count: &Arc<WakeCount>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(&Waker::from(count.clone())))
}
struct ThreadWake {
    thread: thread::Thread,
    notified: AtomicBool,
}
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.notified.store(true, Ordering::Release);
        self.thread.unpark();
    }
}
fn block_on<F: Future>(future: F) -> F::Output {
    let signal = Arc::new(ThreadWake {
        thread: thread::current(),
        notified: AtomicBool::new(false),
    });
    let waker = Waker::from(signal.clone());
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
        // Spurious thread unparks and timeout expiry must never trigger a repoll.
        // Only an actual wake permits another poll; otherwise fail at the deadline.
        while !signal.notified.swap(false, Ordering::Acquire) {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .expect("future stalled or wake was lost");
            thread::park_timeout(remaining);
        }
    }
}

#[test]
fn notification_before_first_poll_is_not_lost() {
    let event = Event::new();
    let mut listener = pin!(event.listen());
    assert_eq!(event.notify(1), 1);
    assert!(poll(listener.as_mut(), &Arc::default()).is_ready());
}
#[test]
fn cancellation_transfers_notification_to_next_waiter() {
    let event = Event::new();
    let first = event.listen();
    let mut second = pin!(event.listen());
    let wake = Arc::<WakeCount>::default();
    assert!(poll(second.as_mut(), &wake).is_pending());
    assert_eq!(event.notify(1), 1);
    assert_eq!(wake.0.load(Ordering::SeqCst), 0);
    drop(first);
    assert!(wake.0.load(Ordering::SeqCst) > 0);
    assert!(poll(second.as_mut(), &wake).is_ready());
}
#[test]
fn listener_registration_is_fifo_and_additional_is_additive() {
    let event = Event::new();
    let mut a = pin!(event.listen());
    let mut b = pin!(event.listen());
    let mut c = pin!(event.listen());
    let w = Arc::default();
    assert_eq!(event.notify(1), 1);
    assert_eq!(event.notify(1), 0);
    assert!(poll(a.as_mut(), &w).is_ready());
    assert!(poll(b.as_mut(), &w).is_pending());
    use async_notification::event::IntoNotification;
    assert_eq!(event.notify(1.additional()), 1);
    assert!(poll(b.as_mut(), &w).is_ready());
    assert!(poll(c.as_mut(), &w).is_pending());
}
#[test]
fn stack_listener_macro_routes_to_shared_event() {
    let event = Event::new();
    async_notification::listener!(event => listener);
    event.notify(1);
    block_on(listener);
}
#[test]
fn pool_style_close_broadcast_reaches_all_registered_waiters() {
    let event = Arc::new(Event::new());
    let registered = Arc::new(std::sync::Barrier::new(9));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let event = event.clone();
        let registered = registered.clone();
        handles.push(thread::spawn(move || {
            let listener = event.listen();
            registered.wait();
            assert!(listener.wait_timeout(Duration::from_secs(5)).is_some());
        }));
    }
    registered.wait();
    assert_eq!(event.notify(usize::MAX), 8);
    for handle in handles {
        handle.join().unwrap();
    }
}
#[test]
fn register_recheck_protocol_survives_repeated_thread_races() {
    for _ in 0..200 {
        let event = Arc::new(Event::new());
        let flag = Arc::new(AtomicUsize::new(0));
        let e = event.clone();
        let f = flag.clone();
        let handle = thread::spawn(move || {
            f.store(1, Ordering::Release);
            e.notify(usize::MAX);
        });
        if flag.load(Ordering::Acquire) == 0 {
            let listener = event.listen();
            if flag.load(Ordering::Acquire) == 0 {
                assert!(listener.wait_timeout(Duration::from_secs(5)).is_some());
            }
        }
        handle.join().unwrap();
        assert_eq!(flag.load(Ordering::Acquire), 1);
    }
}
#[test]
fn mutex_cancellation_does_not_strand_successor() {
    let lock = Mutex::new(0);
    let guard = lock.try_lock().unwrap();
    let w = Arc::default();
    let successor = Arc::default();
    let mut first = Box::pin(lock.lock());
    let mut second = Box::pin(lock.lock());
    assert!(poll(first.as_mut(), &w).is_pending());
    assert!(poll(second.as_mut(), &successor).is_pending());
    w.0.store(0, Ordering::SeqCst);
    drop(guard);
    drop(first);
    assert!(successor.0.load(Ordering::SeqCst) > 0);
    *block_on(second) = 42;
    assert_eq!(*lock.try_lock().unwrap(), 42);
}
#[test]
fn mutex_multithread_contention_keeps_every_update() {
    let lock = Arc::new(Mutex::new(0usize));
    let start = Arc::new(std::sync::Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let lock = lock.clone();
            let start = start.clone();
            thread::spawn(move || {
                start.wait();
                for _ in 0..1000 {
                    let mut guard = block_on(lock.lock());
                    *guard += 1;
                    thread::yield_now();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(*lock.try_lock().unwrap(), 8000);
}
#[test]
fn rwlock_writer_precedes_later_reader() {
    let lock = RwLock::new(0);
    let reader = lock.try_read().unwrap();
    let w = Arc::default();
    let mut writer = Box::pin(lock.write());
    assert!(poll(writer.as_mut(), &w).is_pending());
    let mut later_reader = Box::pin(lock.read());
    assert!(poll(later_reader.as_mut(), &w).is_pending());
    drop(reader);
    let mut guard = match poll(writer.as_mut(), &w) {
        Poll::Ready(x) => x,
        _ => panic!("writer not released"),
    };
    *guard = 19;
    assert!(poll(later_reader.as_mut(), &w).is_pending());
    drop(guard);
    assert_eq!(*block_on(later_reader), 19);
}
#[test]
fn rwlock_cancelled_writer_releases_reader_queue() {
    let lock = RwLock::new(7);
    let reader = lock.try_read().unwrap();
    let w = Arc::default();
    let mut writer = Box::pin(lock.write());
    assert!(poll(writer.as_mut(), &w).is_pending());
    let mut next = Box::pin(lock.read());
    assert!(poll(next.as_mut(), &w).is_pending());
    w.0.store(0, Ordering::SeqCst);
    drop(writer);
    assert!(w.0.load(Ordering::SeqCst) > 0);
    assert_eq!(*block_on(next), 7);
    drop(reader);
}
#[test]
fn owned_rwlock_guard_survives_source_arc_drop() {
    let lock = Arc::new(RwLock::new(5));
    let mut guard = block_on(lock.write_arc());
    drop(lock);
    *guard = 13;
    assert_eq!(*guard, 13);
}
#[test]
fn semaphore_cancelled_waiter_returns_capacity() {
    let semaphore = Semaphore::new(1);
    let permit = semaphore.try_acquire().unwrap();
    let w = Arc::default();
    let successor = Arc::default();
    let mut cancelled = Box::pin(semaphore.acquire());
    let mut next = Box::pin(semaphore.acquire());
    assert!(poll(cancelled.as_mut(), &w).is_pending());
    assert!(poll(next.as_mut(), &successor).is_pending());
    w.0.store(0, Ordering::SeqCst);
    drop(permit);
    drop(cancelled);
    assert!(successor.0.load(Ordering::SeqCst) > 0);
    let next = block_on(next);
    assert!(semaphore.try_acquire().is_none());
    drop(next);
    assert!(semaphore.try_acquire().is_some());
}
#[test]
fn once_cell_cancelled_initializer_can_be_retried() {
    let cell = OnceCell::<usize>::new();
    let w = Arc::default();
    let mut first = Box::pin(cell.get_or_init(|| std::future::pending()));
    assert!(poll(first.as_mut(), &w).is_pending());
    let mut next = Box::pin(cell.get_or_init(|| async { 37 }));
    assert!(poll(next.as_mut(), &w).is_pending());
    w.0.store(0, Ordering::SeqCst);
    drop(first);
    assert!(w.0.load(Ordering::SeqCst) > 0);
    assert_eq!(*block_on(next), 37);
}
